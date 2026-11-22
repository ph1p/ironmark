//! C ABI. The matching header is `include/ironmark.h`; keep both in sync
//! (the `header_matches_rust` test checks the flag values).

use std::ffi::{CString, c_char};

#[cfg(feature = "html")]
use crate::{ParseOptions, render_html};
#[cfg(feature = "html")]
use std::ffi::CStr;

/// `IronmarkOptions::flags` bit: newlines inside paragraphs become `<br />`.
pub const IRONMARK_OPT_HARD_BREAKS: u64 = 1 << 0;
/// `IronmarkOptions::flags` bit: `==highlight==` → `<mark>`.
pub const IRONMARK_OPT_HIGHLIGHT: u64 = 1 << 1;
/// `IronmarkOptions::flags` bit: `~~strikethrough~~` → `<del>`.
pub const IRONMARK_OPT_STRIKETHROUGH: u64 = 1 << 2;
/// `IronmarkOptions::flags` bit: `++underline++` → `<u>`.
pub const IRONMARK_OPT_UNDERLINE: u64 = 1 << 3;
/// `IronmarkOptions::flags` bit: pipe tables.
pub const IRONMARK_OPT_TABLES: u64 = 1 << 4;
/// `IronmarkOptions::flags` bit: link bare URLs and emails.
pub const IRONMARK_OPT_AUTOLINK: u64 = 1 << 5;
/// `IronmarkOptions::flags` bit: `- [ ]` / `- [x]` task lists.
pub const IRONMARK_OPT_TASK_LISTS: u64 = 1 << 6;
/// `IronmarkOptions::flags` bit: escape raw HTML (blocks and inline).
pub const IRONMARK_OPT_DISABLE_RAW_HTML: u64 = 1 << 7;
/// `IronmarkOptions::flags` bit: slug `id=` attributes on headings.
pub const IRONMARK_OPT_HEADING_IDS: u64 = 1 << 8;
/// `IronmarkOptions::flags` bit: `<a class="anchor">` inside headings.
pub const IRONMARK_OPT_HEADING_ANCHORS: u64 = 1 << 9;
/// `IronmarkOptions::flags` bit: 4-space indented code blocks.
pub const IRONMARK_OPT_INDENTED_CODE_BLOCKS: u64 = 1 << 10;
/// `IronmarkOptions::flags` bit: escape HTML blocks only.
pub const IRONMARK_OPT_NO_HTML_BLOCKS: u64 = 1 << 11;
/// `IronmarkOptions::flags` bit: escape inline HTML only.
pub const IRONMARK_OPT_NO_HTML_SPANS: u64 = 1 << 12;
/// `IronmarkOptions::flags` bit: GFM tag filter (`script`, `iframe`, …).
pub const IRONMARK_OPT_TAG_FILTER: u64 = 1 << 13;
/// `IronmarkOptions::flags` bit: collapse runs of spaces/tabs in text.
pub const IRONMARK_OPT_COLLAPSE_WHITESPACE: u64 = 1 << 14;
/// `IronmarkOptions::flags` bit: allow `#Heading` without a space.
pub const IRONMARK_OPT_PERMISSIVE_ATX_HEADERS: u64 = 1 << 15;
/// `IronmarkOptions::flags` bit: `[[wiki link]]` syntax.
pub const IRONMARK_OPT_WIKI_LINKS: u64 = 1 << 16;
/// `IronmarkOptions::flags` bit: `$inline$` / `$$display$$` math.
pub const IRONMARK_OPT_LATEX_MATH: u64 = 1 << 17;
/// `IronmarkOptions::flags` bit: MDX syntax. MDX is code: trusted authors only.
pub const IRONMARK_OPT_MDX: u64 = 1 << 18;
/// `IronmarkOptions::flags` bit: skip a leading `---`/`+++` front matter block.
pub const IRONMARK_OPT_FRONTMATTER: u64 = 1 << 19;

/// Every flag bit this version understands.
pub const IRONMARK_OPT_ALL: u64 = (1 << 20) - 1;

/// Options for [`ironmark_render_html_with_options`]. Start from
/// [`ironmark_options_default`] and toggle bits, so new fields and flags
/// keep their defaults.
#[cfg(feature = "html")]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IronmarkOptions {
    /// Bitwise OR of `IRONMARK_OPT_*` values.
    pub flags: u64,
    /// See [`ParseOptions::max_nesting_depth`].
    pub max_nesting_depth: usize,
    /// See [`ParseOptions::max_input_size`]; `0` = unlimited.
    pub max_input_size: usize,
}

#[cfg(feature = "html")]
impl IronmarkOptions {
    fn from_parse_options(o: &ParseOptions) -> Self {
        let bit = |on: bool, b: u64| if on { b } else { 0 };
        Self {
            flags: bit(o.hard_breaks, IRONMARK_OPT_HARD_BREAKS)
                | bit(o.enable_highlight, IRONMARK_OPT_HIGHLIGHT)
                | bit(o.enable_strikethrough, IRONMARK_OPT_STRIKETHROUGH)
                | bit(o.enable_underline, IRONMARK_OPT_UNDERLINE)
                | bit(o.enable_tables, IRONMARK_OPT_TABLES)
                | bit(o.enable_autolink, IRONMARK_OPT_AUTOLINK)
                | bit(o.enable_task_lists, IRONMARK_OPT_TASK_LISTS)
                | bit(o.disable_raw_html, IRONMARK_OPT_DISABLE_RAW_HTML)
                | bit(o.enable_heading_ids, IRONMARK_OPT_HEADING_IDS)
                | bit(o.enable_heading_anchors, IRONMARK_OPT_HEADING_ANCHORS)
                | bit(
                    o.enable_indented_code_blocks,
                    IRONMARK_OPT_INDENTED_CODE_BLOCKS,
                )
                | bit(o.no_html_blocks, IRONMARK_OPT_NO_HTML_BLOCKS)
                | bit(o.no_html_spans, IRONMARK_OPT_NO_HTML_SPANS)
                | bit(o.tag_filter, IRONMARK_OPT_TAG_FILTER)
                | bit(o.collapse_whitespace, IRONMARK_OPT_COLLAPSE_WHITESPACE)
                | bit(
                    o.permissive_atx_headers,
                    IRONMARK_OPT_PERMISSIVE_ATX_HEADERS,
                )
                | bit(o.enable_wiki_links, IRONMARK_OPT_WIKI_LINKS)
                | bit(o.enable_latex_math, IRONMARK_OPT_LATEX_MATH)
                | bit(o.enable_mdx, IRONMARK_OPT_MDX)
                | bit(o.enable_frontmatter, IRONMARK_OPT_FRONTMATTER),
            max_nesting_depth: o.max_nesting_depth,
            max_input_size: o.max_input_size,
        }
    }

    /// `None` when `flags` has bits this version does not know: silently
    /// ignoring a newer safety flag would be worse than failing.
    fn to_parse_options(self) -> Option<ParseOptions> {
        if self.flags & !IRONMARK_OPT_ALL != 0 {
            return None;
        }
        let on = |b: u64| self.flags & b != 0;
        Some(ParseOptions {
            hard_breaks: on(IRONMARK_OPT_HARD_BREAKS),
            enable_highlight: on(IRONMARK_OPT_HIGHLIGHT),
            enable_strikethrough: on(IRONMARK_OPT_STRIKETHROUGH),
            enable_underline: on(IRONMARK_OPT_UNDERLINE),
            enable_tables: on(IRONMARK_OPT_TABLES),
            enable_autolink: on(IRONMARK_OPT_AUTOLINK),
            enable_task_lists: on(IRONMARK_OPT_TASK_LISTS),
            disable_raw_html: on(IRONMARK_OPT_DISABLE_RAW_HTML),
            max_nesting_depth: self.max_nesting_depth,
            max_input_size: self.max_input_size,
            enable_heading_ids: on(IRONMARK_OPT_HEADING_IDS),
            enable_heading_anchors: on(IRONMARK_OPT_HEADING_ANCHORS),
            enable_indented_code_blocks: on(IRONMARK_OPT_INDENTED_CODE_BLOCKS),
            no_html_blocks: on(IRONMARK_OPT_NO_HTML_BLOCKS),
            no_html_spans: on(IRONMARK_OPT_NO_HTML_SPANS),
            tag_filter: on(IRONMARK_OPT_TAG_FILTER),
            collapse_whitespace: on(IRONMARK_OPT_COLLAPSE_WHITESPACE),
            permissive_atx_headers: on(IRONMARK_OPT_PERMISSIVE_ATX_HEADERS),
            enable_wiki_links: on(IRONMARK_OPT_WIKI_LINKS),
            enable_latex_math: on(IRONMARK_OPT_LATEX_MATH),
            enable_mdx: on(IRONMARK_OPT_MDX),
            enable_frontmatter: on(IRONMARK_OPT_FRONTMATTER),
        })
    }
}

#[cfg(feature = "html")]
fn into_c_string(html: String) -> *mut c_char {
    match CString::new(html) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Parse markdown input and return a heap-allocated HTML string.
///
/// The caller **must** free the returned pointer with [`ironmark_free`].
/// Returns a null pointer if the input is null or contains invalid UTF-8.
///
/// # Safety
///
/// `input` must be a valid, null-terminated C string.
#[cfg(feature = "html")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ironmark_render_html(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: caller guarantees `input` is a valid null-terminated C string.
    let c_str = unsafe { CStr::from_ptr(input) };
    let Ok(markdown) = c_str.to_str() else {
        return std::ptr::null_mut();
    };
    into_c_string(render_html(markdown, &ParseOptions::default()))
}

/// The options [`ironmark_render_html`] uses.
#[cfg(feature = "html")]
#[unsafe(no_mangle)]
pub extern "C" fn ironmark_options_default() -> IronmarkOptions {
    IronmarkOptions::from_parse_options(&ParseOptions::default())
}

/// Parse `len` bytes of markdown with `options` and return a heap-allocated
/// HTML string. The input needs no terminating NUL.
///
/// `options` may be null for the defaults. Returns null if `input` is null,
/// the input is not valid UTF-8, or `options->flags` has unknown bits.
/// The caller **must** free the result with [`ironmark_free`].
///
/// # Safety
///
/// `input` must point to `len` readable bytes. `options` must be null or
/// point to a valid [`IronmarkOptions`].
#[cfg(feature = "html")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ironmark_render_html_with_options(
    input: *const c_char,
    len: usize,
    options: *const IronmarkOptions,
) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: caller guarantees `input` points to `len` readable bytes, and
    // it is non-null (checked above).
    let bytes = unsafe { std::slice::from_raw_parts(input.cast::<u8>(), len) };
    let Ok(markdown) = std::str::from_utf8(bytes) else {
        return std::ptr::null_mut();
    };
    let opts = if options.is_null() {
        ParseOptions::default()
    } else {
        // SAFETY: caller guarantees a non-null `options` points to a valid
        // `IronmarkOptions`; it is `Copy`, so reading it moves nothing.
        match unsafe { *options }.to_parse_options() {
            Some(o) => o,
            None => return std::ptr::null_mut(),
        }
    };
    into_c_string(render_html(markdown, &opts))
}

/// Free a string returned by [`ironmark_render_html`] or
/// [`ironmark_render_html_with_options`].
///
/// # Safety
///
/// `ptr` must be a pointer returned by one of those functions, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ironmark_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: `ptr` was produced by `CString::into_raw` in a render function.
        drop(unsafe { CString::from_raw(ptr) });
    }
}

#[cfg(all(test, feature = "html"))]
mod tests {
    use super::*;

    const FLAGS: [(&str, u64); 20] = [
        ("HARD_BREAKS", IRONMARK_OPT_HARD_BREAKS),
        ("HIGHLIGHT", IRONMARK_OPT_HIGHLIGHT),
        ("STRIKETHROUGH", IRONMARK_OPT_STRIKETHROUGH),
        ("UNDERLINE", IRONMARK_OPT_UNDERLINE),
        ("TABLES", IRONMARK_OPT_TABLES),
        ("AUTOLINK", IRONMARK_OPT_AUTOLINK),
        ("TASK_LISTS", IRONMARK_OPT_TASK_LISTS),
        ("DISABLE_RAW_HTML", IRONMARK_OPT_DISABLE_RAW_HTML),
        ("HEADING_IDS", IRONMARK_OPT_HEADING_IDS),
        ("HEADING_ANCHORS", IRONMARK_OPT_HEADING_ANCHORS),
        ("INDENTED_CODE_BLOCKS", IRONMARK_OPT_INDENTED_CODE_BLOCKS),
        ("NO_HTML_BLOCKS", IRONMARK_OPT_NO_HTML_BLOCKS),
        ("NO_HTML_SPANS", IRONMARK_OPT_NO_HTML_SPANS),
        ("TAG_FILTER", IRONMARK_OPT_TAG_FILTER),
        ("COLLAPSE_WHITESPACE", IRONMARK_OPT_COLLAPSE_WHITESPACE),
        (
            "PERMISSIVE_ATX_HEADERS",
            IRONMARK_OPT_PERMISSIVE_ATX_HEADERS,
        ),
        ("WIKI_LINKS", IRONMARK_OPT_WIKI_LINKS),
        ("LATEX_MATH", IRONMARK_OPT_LATEX_MATH),
        ("MDX", IRONMARK_OPT_MDX),
        ("FRONTMATTER", IRONMARK_OPT_FRONTMATTER),
    ];

    fn render(md: &str, opts: Option<&IronmarkOptions>) -> Option<String> {
        let p = opts.map_or(std::ptr::null(), |o| o as *const _);
        // SAFETY: `md` is a live str of `md.len()` bytes; `p` is null or a live reference.
        let out = unsafe { ironmark_render_html_with_options(md.as_ptr().cast(), md.len(), p) };
        if out.is_null() {
            return None;
        }
        // SAFETY: non-null `out` is a NUL-terminated string from `CString::into_raw`.
        let s = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_owned();
        // SAFETY: `out` came from the render function and is freed once.
        unsafe { ironmark_free(out) };
        Some(s)
    }

    #[test]
    fn flags_cover_every_bit_once() {
        let mut seen = 0;
        for (_, bit) in FLAGS {
            assert_eq!(bit.count_ones(), 1);
            assert_eq!(seen & bit, 0);
            seen |= bit;
        }
        assert_eq!(seen, IRONMARK_OPT_ALL);
    }

    #[test]
    fn flags_round_trip_through_parse_options() {
        for (name, bit) in FLAGS {
            for flags in [bit, IRONMARK_OPT_ALL & !bit] {
                let o = IronmarkOptions {
                    flags,
                    max_nesting_depth: 7,
                    max_input_size: 9,
                };
                let back = IronmarkOptions::from_parse_options(&o.to_parse_options().unwrap());
                assert_eq!(back, o, "{name}");
            }
        }
    }

    #[test]
    fn default_matches_parse_options_default() {
        let o = ironmark_options_default();
        assert_eq!(o.to_parse_options().unwrap(), ParseOptions::default());
    }

    #[test]
    fn header_matches_rust() {
        let header = include_str!("../include/ironmark.h");
        for (name, bit) in FLAGS {
            let line = format!(
                "#define IRONMARK_OPT_{name} (UINT64_C(1) << {})",
                bit.trailing_zeros()
            );
            assert!(header.contains(&line), "header is missing `{line}`");
        }
        let all = format!(
            "#define IRONMARK_OPT_ALL ((UINT64_C(1) << {}) - 1)",
            IRONMARK_OPT_ALL.count_ones()
        );
        assert!(header.contains(&all), "header is missing `{all}`");
    }

    #[test]
    fn null_options_use_defaults() {
        let md = "a\nb ~~c~~";
        assert_eq!(
            render(md, None).unwrap(),
            render_html(md, &ParseOptions::default())
        );
    }

    #[test]
    fn options_are_applied() {
        let mut o = ironmark_options_default();
        o.flags |= IRONMARK_OPT_DISABLE_RAW_HTML;
        o.flags &= !IRONMARK_OPT_STRIKETHROUGH;
        let html = render("<b>x</b> ~~y~~", Some(&o)).unwrap();
        assert!(html.contains("&lt;b&gt;"), "{html}");
        assert!(!html.contains("<del>"), "{html}");
    }

    #[test]
    fn input_is_length_bounded() {
        // No NUL terminator needed; bytes past `len` are not read.
        let buf = b"# Hi\nignored";
        // SAFETY: `buf` holds at least 5 bytes.
        let out =
            unsafe { ironmark_render_html_with_options(buf.as_ptr().cast(), 5, std::ptr::null()) };
        // SAFETY: non-null result from `CString::into_raw`.
        let s = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_owned();
        // SAFETY: freed once.
        unsafe { ironmark_free(out) };
        assert_eq!(s, "<h1>Hi</h1>\n");
    }

    #[test]
    fn rejects_bad_input() {
        let mut o = ironmark_options_default();
        o.flags |= 1 << 63;
        assert_eq!(render("x", Some(&o)), None);
        let bad = [0xffu8, 0xfe];
        // SAFETY: `bad` holds 2 bytes.
        let out =
            unsafe { ironmark_render_html_with_options(bad.as_ptr().cast(), 2, std::ptr::null()) };
        assert!(out.is_null());
        // SAFETY: null input is allowed and returns null.
        let out =
            unsafe { ironmark_render_html_with_options(std::ptr::null(), 0, std::ptr::null()) };
        assert!(out.is_null());
    }

    #[test]
    fn embedded_nul_does_not_fail() {
        // CommonMark replaces U+0000 with U+FFFD, so the result is a valid C string.
        assert!(render("a\0b", None).is_some());
    }
}
