//! MDX syntax support: JSX tags, `{expressions}` and top-level `import`/`export`.
//!
//! The lexers here are deliberately small. They recognise where an MDX
//! construct starts and ends without understanding JavaScript: expressions are
//! matched by brace depth while skipping strings, template literals, comments
//! and nested JSX elements. Regular-expression literals that contain an
//! unbalanced brace are not understood (a known limitation).
//!
//! The block parser and inline scanner call into this module only when
//! [`ParseOptions::enable_mdx`](crate::ParseOptions::enable_mdx) is set.

#[cfg(feature = "html")]
pub(crate) mod convert;

/// Maximum nesting of JSX elements inside expressions inside JSX elements.
/// Deeper input is rejected instead of risking stack exhaustion.
const MAX_DEPTH: usize = 64;

/// Result of lexing one MDX construct.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Lex {
    /// The construct ends at this byte offset (exclusive).
    Done(usize),
    /// The input ended before the construct was closed.
    Incomplete,
    /// The bytes do not form this construct.
    Invalid,
}

/// Kind of a lexed JSX tag.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum TagKind {
    Open,
    Close,
    SelfClosing,
}

impl TagKind {
    /// Single-byte marker stored in sentinel-wrapped output.
    pub(crate) fn marker(self, flow: bool) -> char {
        let m = match self {
            TagKind::Open => 'o',
            TagKind::Close => 'c',
            TagKind::SelfClosing => 's',
        };
        if flow { m.to_ascii_uppercase() } else { m }
    }
}

#[inline]
fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

#[inline]
fn is_id_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$' || b >= 0x80
}

#[inline]
fn is_id_continue(b: u8) -> bool {
    is_id_start(b) || b.is_ascii_digit() || b == b'-'
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && is_ws(b[i]) {
        i += 1;
    }
    i
}

/// Skip whitespace and JS comments (allowed between attributes).
fn skip_ws_comments(b: &[u8], mut i: usize) -> Result<usize, Lex> {
    loop {
        i = skip_ws(b, i);
        if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'*' {
            match memchr::memmem::find(&b[i + 2..], b"*/") {
                Some(off) => i = i + 2 + off + 2,
                None => return Err(Lex::Incomplete),
            }
        } else if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'/' {
            match memchr::memchr(b'\n', &b[i..]) {
                Some(off) => i += off + 1,
                None => return Err(Lex::Incomplete),
            }
        } else {
            return Ok(i);
        }
    }
}

/// Lex an identifier (JSX flavour: `-` allowed after the first byte).
fn lex_ident(b: &[u8], i: usize) -> Option<usize> {
    if i >= b.len() || !is_id_start(b[i]) {
        return None;
    }
    let mut j = i + 1;
    while j < b.len() && is_id_continue(b[j]) {
        j += 1;
    }
    Some(j)
}

/// Lex a tag name: `a`, `a.b.c` (member) or `a:b` (namespace).
fn lex_tag_name(b: &[u8], i: usize) -> Result<usize, Lex> {
    let mut j = match lex_ident(b, i) {
        Some(j) => j,
        None if i >= b.len() => return Err(Lex::Incomplete),
        None => return Err(Lex::Invalid),
    };
    if j < b.len() && b[j] == b':' {
        return lex_ident(b, j + 1).ok_or(Lex::Invalid);
    }
    while j < b.len() && b[j] == b'.' {
        j = lex_ident(b, j + 1).ok_or(Lex::Invalid)?;
    }
    Ok(j)
}

/// Lex one JSX tag starting at `b[start] == b'<'`: an opening tag, a closing
/// tag, a self-closing tag, or a fragment (`<>` / `</>`).
pub(crate) fn lex_jsx_tag(b: &[u8], start: usize) -> Result<(usize, TagKind), Lex> {
    lex_tag_at(b, start, 0)
}

/// `Lex::Done(end)` as `Ok(end)`, anything else as the error.
#[inline]
fn done(l: Lex) -> Result<usize, Lex> {
    match l {
        Lex::Done(e) => Ok(e),
        other => Err(other),
    }
}

fn lex_tag_at(b: &[u8], start: usize, depth: usize) -> Result<(usize, TagKind), Lex> {
    debug_assert_eq!(b[start], b'<');
    let len = b.len();
    let mut i = start + 1;
    // The first byte after `<` must be a name start, `>`, `/` — never a space.
    // This keeps prose like `a < b` from being lexed as a tag.
    if i >= len {
        return Err(Lex::Incomplete);
    }
    if !(is_id_start(b[i]) || b[i] == b'>' || b[i] == b'/') {
        return Err(Lex::Invalid);
    }
    let closing = b[i] == b'/';
    if closing {
        i = skip_ws_comments(b, i + 1)?;
        if i >= len {
            return Err(Lex::Incomplete);
        }
        if b[i] == b'>' {
            return Ok((i + 1, TagKind::Close));
        }
        i = lex_tag_name(b, i)?;
        i = skip_ws_comments(b, i)?;
        return match b.get(i) {
            None => Err(Lex::Incomplete),
            Some(b'>') => Ok((i + 1, TagKind::Close)),
            Some(_) => Err(Lex::Invalid),
        };
    }
    if b[i] == b'>' {
        return Ok((i + 1, TagKind::Open));
    }
    i = lex_tag_name(b, i)?;
    loop {
        i = skip_ws_comments(b, i)?;
        let Some(&c) = b.get(i) else {
            return Err(Lex::Incomplete);
        };
        match c {
            b'>' => return Ok((i + 1, TagKind::Open)),
            b'/' => {
                i = skip_ws_comments(b, i + 1)?;
                return match b.get(i) {
                    None => Err(Lex::Incomplete),
                    Some(b'>') => Ok((i + 1, TagKind::SelfClosing)),
                    Some(_) => Err(Lex::Invalid),
                };
            }
            b'{' => {
                // Spread attribute: `{...props}`.
                i = done(lex_expr_at(b, i, depth + 1))?;
            }
            _ => {
                i = lex_ident(b, i).ok_or(Lex::Invalid)?;
                if i < len && b[i] == b':' {
                    i = lex_ident(b, i + 1).ok_or(Lex::Invalid)?;
                }
                let after = skip_ws_comments(b, i)?;
                if after < len && b[after] == b'=' {
                    i = skip_ws_comments(b, after + 1)?;
                    let Some(&q) = b.get(i) else {
                        return Err(Lex::Incomplete);
                    };
                    match q {
                        b'"' | b'\'' => match memchr::memchr(q, &b[i + 1..]) {
                            Some(off) => i = i + 1 + off + 1,
                            None => return Err(Lex::Incomplete),
                        },
                        b'{' => {
                            i = done(lex_expr_at(b, i, depth + 1))?;
                        }
                        b'<' => {
                            i = done(lex_element_at(b, i, depth + 1))?;
                        }
                        _ => return Err(Lex::Invalid),
                    }
                }
            }
        }
    }
}

/// Lex a whole JSX element (opening tag, children, closing tag) inside an
/// expression or attribute value.
fn lex_element_at(b: &[u8], start: usize, depth: usize) -> Lex {
    if depth > MAX_DEPTH {
        return Lex::Invalid;
    }
    let (mut i, kind) = match lex_tag_at(b, start, depth) {
        Ok(t) => t,
        Err(e) => return e,
    };
    match kind {
        TagKind::SelfClosing => return Lex::Done(i),
        TagKind::Close => return Lex::Invalid,
        TagKind::Open => {}
    }
    let mut open = 1usize;
    while i < b.len() {
        match b[i] {
            b'{' => match lex_expr_at(b, i, depth + 1) {
                Lex::Done(e) => i = e,
                other => return other,
            },
            b'<' => {
                match lex_tag_at(b, i, depth + 1) {
                    Ok((e, kind)) => {
                        i = e;
                        match kind {
                            TagKind::Open => open += 1,
                            TagKind::Close => {
                                open -= 1;
                                if open == 0 {
                                    return Lex::Done(i);
                                }
                            }
                            TagKind::SelfClosing => {}
                        }
                    }
                    // `<` that is not a tag is plain JSX text here.
                    Err(Lex::Invalid) => i += 1,
                    Err(e) => return e,
                }
            }
            _ => i += 1,
        }
    }
    Lex::Incomplete
}

/// Lex a `{…}` expression starting at `b[start] == b'{'`.
pub(crate) fn lex_expression(b: &[u8], start: usize) -> Lex {
    lex_expr_at(b, start, 0)
}

fn lex_expr_at(b: &[u8], start: usize, depth: usize) -> Lex {
    debug_assert_eq!(b[start], b'{');
    if depth > MAX_DEPTH {
        return Lex::Invalid;
    }
    // Stack of open contexts: `{` (code) or `` ` `` (template literal body).
    let mut stack: smallvec::SmallVec<[u8; 8]> = smallvec::smallvec![b'{'];
    let len = b.len();
    let mut i = start + 1;
    // Last non-whitespace code byte, used to decide whether `<` starts JSX.
    let mut prev: u8 = b'{';
    while i < len {
        let c = b[i];
        if *stack.last().unwrap() == b'`' {
            match c {
                b'\\' => i += 2,
                b'`' => {
                    stack.pop();
                    prev = b'`';
                    i += 1;
                }
                b'$' if i + 1 < len && b[i + 1] == b'{' => {
                    stack.push(b'{');
                    prev = b'{';
                    i += 2;
                }
                _ => i += 1,
            }
            continue;
        }
        match c {
            b'"' | b'\'' => {
                let mut j = i + 1;
                loop {
                    if j >= len {
                        return Lex::Incomplete;
                    }
                    match b[j] {
                        b'\\' => j += 2,
                        x if x == c => break,
                        _ => j += 1,
                    }
                }
                i = j + 1;
                prev = c;
            }
            b'`' => {
                stack.push(b'`');
                i += 1;
            }
            b'/' if i + 1 < len && b[i + 1] == b'/' => match memchr::memchr(b'\n', &b[i..]) {
                Some(off) => i += off + 1,
                None => return Lex::Incomplete,
            },
            b'/' if i + 1 < len && b[i + 1] == b'*' => {
                match memchr::memmem::find(&b[i + 2..], b"*/") {
                    Some(off) => i = i + 2 + off + 2,
                    None => return Lex::Incomplete,
                }
            }
            b'{' => {
                stack.push(b'{');
                prev = b'{';
                i += 1;
            }
            b'}' => {
                stack.pop();
                if stack.is_empty() {
                    return Lex::Done(i + 1);
                }
                prev = b'}';
                i += 1;
            }
            b'<' if matches!(
                prev,
                b'(' | b',' | b'=' | b':' | b'?' | b'&' | b'|' | b'{' | b'[' | b'!' | b';' | b'>'
            ) && i + 1 < len
                && (is_id_start(b[i + 1]) || b[i + 1] == b'>') =>
            {
                match lex_element_at(b, i, depth + 1) {
                    Lex::Done(e) => {
                        i = e;
                        prev = b')';
                    }
                    // Not JSX after all (e.g. a comparison): treat as an operator.
                    Lex::Invalid => {
                        prev = b'<';
                        i += 1;
                    }
                    Lex::Incomplete => return Lex::Incomplete,
                }
            }
            _ => {
                if !is_ws(c) {
                    prev = c;
                }
                i += 1;
            }
        }
    }
    Lex::Incomplete
}

/// One MDX piece found on a flow line.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Piece {
    Tag(TagKind),
    Expr,
}

/// Lex a run of JSX tags and expressions separated by whitespace that makes up
/// a whole flow construct: after the last piece only spaces/tabs may follow on
/// its line. `b` starts at the first piece. On success returns the end offset
/// of the last piece (trailing whitespace is not included).
///
/// `on_piece` receives each piece's byte range and kind.
pub(crate) fn lex_flow(b: &[u8], mut on_piece: impl FnMut(usize, usize, Piece)) -> Lex {
    let mut i = 0;
    let mut count = 0usize;
    let mut last_end = 0;
    loop {
        while i < b.len() && matches!(b[i], b' ' | b'\t') {
            i += 1;
        }
        if i >= b.len() || b[i] == b'\n' || b[i] == b'\r' {
            if count == 0 {
                return Lex::Invalid;
            }
            return Lex::Done(last_end);
        }
        let lexed = match b[i] {
            b'<' => lex_jsx_tag(b, i).map(|(e, kind)| (e, Piece::Tag(kind))),
            b'{' => done(lex_expression(b, i)).map(|e| (e, Piece::Expr)),
            _ => return Lex::Invalid,
        };
        let (e, piece) = match lexed {
            Ok(t) => t,
            Err(other) => return other,
        };
        on_piece(i, e, piece);
        count += 1;
        last_end = e;
        i = e;
    }
}

/// Whether `line` (at column 0 of a top-level line) starts an ESM block:
/// `import` or `export` followed by whitespace, `{` or `*`.
pub(crate) fn is_esm_start(line: &str) -> bool {
    let b = line.as_bytes();
    (b.starts_with(b"import") || b.starts_with(b"export"))
        && b.get(6)
            .is_some_and(|&c| matches!(c, b' ' | b'\t' | b'{' | b'*'))
}

/// Sentinel byte that brackets MDX pieces in HTML produced for the JSX
/// converter. Real NUL bytes in the input are replaced with U+FFFD first, so
/// this byte can never come from the document.
pub(crate) const SENTINEL: char = '\0';

/// Append one sentinel-wrapped piece: `\0`, marker, literal, `\0`.
#[inline]
pub(crate) fn push_sentinel(out: &mut String, marker: char, literal: &str) {
    out.push(SENTINEL);
    out.push(marker);
    out.push_str(literal);
    out.push(SENTINEL);
}

/// Append an MDX literal in HTML mode: raw, or escaped under `disable_raw_html`
/// (`no_html_blocks` is forced on by the MDX preset and must not affect it).
#[inline]
pub(crate) fn push_raw(out: &mut String, literal: &str, escape: bool) {
    if escape {
        crate::html::escape_html_into(out, literal);
    } else {
        out.push_str(literal);
    }
}

/// Re-lex a flow block literal and append each piece wrapped in a flow sentinel,
/// keeping the whitespace between pieces as-is.
pub(crate) fn push_flow_sentinels(out: &mut String, literal: &str) {
    let mut last = 0;
    let res = lex_flow(literal.as_bytes(), |s, e, piece| {
        out.push_str(&literal[last..s]);
        let marker = match piece {
            Piece::Tag(kind) => kind.marker(true),
            Piece::Expr => 'E',
        };
        push_sentinel(out, marker, &literal[s..e]);
        last = e;
    });
    // A block that never completed (unterminated at end of input) keeps its
    // tail as an unterminated piece so the converter can report it.
    if !matches!(res, Lex::Done(_)) && last < literal.len() {
        push_sentinel(out, 'X', &literal[last..]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(s: &str) -> (Lex, TagKind) {
        match lex_jsx_tag(s.as_bytes(), 0) {
            Ok((e, kind)) => (Lex::Done(e), kind),
            Err(e) => (e, TagKind::Open),
        }
    }

    #[test]
    fn tags() {
        assert_eq!(tag("<a>"), (Lex::Done(3), TagKind::Open));
        assert_eq!(tag("<a/>x"), (Lex::Done(4), TagKind::SelfClosing));
        assert_eq!(tag("</a >"), (Lex::Done(5), TagKind::Close));
        assert_eq!(tag("<>"), (Lex::Done(2), TagKind::Open));
        assert_eq!(tag("</>"), (Lex::Done(3), TagKind::Close));
        assert_eq!(
            tag("<a.b.c d=\"1\" e='2' f={3} {...g} h />").0,
            Lex::Done(36)
        );
        assert_eq!(tag("<svg:rect />").0, Lex::Done(12));
        assert_eq!(tag("<a\n  b=\"1\"\n>").0, Lex::Done(12));
        assert_eq!(tag("<a b={<c />} />").0, Lex::Done(15));
        assert_eq!(tag("< a>").0, Lex::Invalid);
        assert_eq!(tag("<1>").0, Lex::Invalid);
        assert_eq!(tag("<a b=c>").0, Lex::Invalid);
        assert_eq!(tag("<a b=\"1").0, Lex::Incomplete);
        assert_eq!(tag("<a").0, Lex::Incomplete);
    }

    #[test]
    fn expressions() {
        let e = |s: &str| lex_expression(s.as_bytes(), 0);
        assert_eq!(e("{a}"), Lex::Done(3));
        assert_eq!(e("{ {a: 1} }x"), Lex::Done(10));
        assert_eq!(e("{'}'}"), Lex::Done(5));
        assert_eq!(e("{`${'}'}`}"), Lex::Done(10));
        assert_eq!(e("{/* } */ a}"), Lex::Done(11));
        assert_eq!(e("{a // }\n}"), Lex::Done(9));
        assert_eq!(e("{x && <p>don't {y}</p>}"), Lex::Done(23));
        assert_eq!(e("{a < b}"), Lex::Done(7));
        assert_eq!(e("{a"), Lex::Incomplete);
    }

    #[test]
    fn flow() {
        let f = |s: &str| lex_flow(s.as_bytes(), |_, _, _| {});
        assert_eq!(f("<a> <b />  "), Lex::Done(9));
        assert_eq!(f("{x}"), Lex::Done(3));
        assert_eq!(f("<a>text"), Lex::Invalid);
        assert_eq!(f("<a\n"), Lex::Incomplete);
    }

    #[test]
    fn esm() {
        assert!(is_esm_start("import x from 'y'"));
        assert!(is_esm_start("export const a = 1"));
        assert!(is_esm_start("import{a} from 'b'"));
        assert!(!is_esm_start("important"));
        assert!(!is_esm_start("exports"));
    }
}
