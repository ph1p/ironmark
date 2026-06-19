use super::constants::RESET;

/// Expand tab characters in `s` to spaces using `tab_width`-column tab stops.
pub(super) fn expand_tabs(s: &str, tab_width: usize) -> String {
    if !s.contains('\t') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len() + 8);
    let mut col = 0usize;
    for ch in s.chars() {
        if ch == '\t' {
            let spaces = tab_width - (col % tab_width);
            for _ in 0..spaces {
                out.push(' ');
            }
            col += spaces;
        } else {
            out.push(ch);
            col += 1;
        }
    }
    out
}

/// Word-wrap ANSI-tagged text to `max_cols` visible columns.
///
/// - Splits on space boundaries in the visible text.
/// - Words longer than `max_cols` are character-wrapped to fit.
/// - ANSI escape sequences are treated as zero-width and never broken.
/// - Continuation lines are prefixed with `indent`.
/// - Emits `RESET` before each line break so active backgrounds don't bleed to
///   end-of-line, then re-emits accumulated ANSI state at the start of the next line.
/// - When `max_cols` is 0 the text is returned unchanged.
pub(super) fn wrap_ansi(text: &str, max_cols: usize, indent: &str) -> String {
    if max_cols == 0 || visible_len(text) <= max_cols {
        return text.to_owned();
    }

    let mut out = String::with_capacity(text.len() + 32);
    let indent_cols = visible_len(indent);
    let mut line_cols: usize = 0;
    let mut line_has_ansi = false;
    // Track all ANSI CSI sequences seen so far so we can re-emit them after a
    // line break (prevents background colour bleeding to end-of-line).
    let mut ansi_state = String::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    // Collect one space-delimited token (including trailing spaces and interleaved ANSI).
    let collect_word = |start: usize| -> (String, usize, usize) {
        let mut w = String::new();
        let mut vis = 0usize;
        let mut j = start;
        let mut in_spaces = false;
        while j < len {
            if bytes[j] == b'\x1b' {
                w.push('\x1b');
                j += 1;
                if j < len && bytes[j] == b'[' {
                    w.push('[');
                    j += 1;
                    while j < len && !(0x40..=0x7e).contains(&bytes[j]) {
                        w.push(bytes[j] as char);
                        j += 1;
                    }
                    if j < len {
                        w.push(bytes[j] as char);
                        j += 1;
                    }
                } else if j < len && bytes[j] == b']' {
                    // OSC sequence: ESC ] ... ESC \  or  ESC ] ... BEL
                    w.push(']');
                    j += 1;
                    while j < len {
                        let b = bytes[j];
                        w.push(b as char);
                        j += 1;
                        if b == 0x07 {
                            break; // BEL terminator
                        }
                        if b == b'\x1b' && j < len && bytes[j] == b'\\' {
                            w.push('\\');
                            j += 1;
                            break; // ST terminator
                        }
                    }
                }
            } else if bytes[j] == b' ' {
                w.push(' ');
                vis += 1;
                j += 1;
                in_spaces = true;
            } else {
                if in_spaces {
                    break;
                }
                let char_len = crate::utf8_char_len(bytes[j]);
                let c = text[j..].chars().next().unwrap_or('\u{FFFD}');
                w.push_str(&text[j..j + char_len]);
                vis += char_width(c);
                j += char_len;
            }
        }
        (w, vis, j)
    };

    // Emit word char-by-char when too long to fit on a line.
    let emit_long_word = |out: &mut String,
                          word: &str,
                          line_cols: &mut usize,
                          line_has_ansi: &mut bool,
                          ansi_state: &mut String,
                          max_cols: usize,
                          indent: &str,
                          indent_cols: usize| {
        let wb = word.as_bytes();
        let mut j = 0;

        // Wrap to next line if at capacity
        let maybe_wrap = |out: &mut String,
                          line_cols: &mut usize,
                          line_has_ansi: &bool,
                          ansi_state: &str,
                          indent_cols: usize| {
            if *line_cols >= max_cols {
                if *line_has_ansi {
                    out.push_str(RESET);
                }
                out.push('\n');
                out.push_str(indent);
                if !ansi_state.is_empty() {
                    out.push_str(ansi_state);
                }
                *line_cols = indent_cols;
            }
        };

        while j < wb.len() {
            if wb[j] == b'\x1b' {
                let seq_start = j;
                j += 1;
                if j < wb.len() && wb[j] == b'[' {
                    j += 1;
                    while j < wb.len() && !(0x40..=0x7e).contains(&wb[j]) {
                        j += 1;
                    }
                    if j < wb.len() {
                        j += 1;
                    }
                    let seq = &word[seq_start..j];
                    out.push_str(seq);
                    *line_has_ansi = true;
                    if seq == "\x1b[0m" || seq == "\x1b[m" {
                        ansi_state.clear();
                    } else {
                        ansi_state.push_str(seq);
                    }
                } else if j < wb.len() && wb[j] == b']' {
                    j += 1;
                    while j < wb.len() {
                        let b = wb[j];
                        j += 1;
                        if b == 0x07 {
                            break;
                        }
                        if b == b'\x1b' && j < wb.len() && wb[j] == b'\\' {
                            j += 1;
                            break;
                        }
                    }
                    out.push_str(&word[seq_start..j]);
                }
            } else if wb[j] == b' ' {
                maybe_wrap(out, line_cols, line_has_ansi, ansi_state, indent_cols);
                out.push(' ');
                *line_cols += 1;
                j += 1;
            } else {
                maybe_wrap(out, line_cols, line_has_ansi, ansi_state, indent_cols);
                let char_len = crate::utf8_char_len(wb[j]);
                let c = word[j..].chars().next().unwrap_or('\u{FFFD}');
                out.push_str(&word[j..j + char_len]);
                *line_cols += char_width(c);
                j += char_len;
            }
        }
    };

    while i < len {
        let (word, word_vis, next_i) = collect_word(i);
        if word.is_empty() {
            break;
        }
        let word_has_ansi = word.contains('\x1b');

        // Check if word fits on current line
        if line_cols > indent_cols && line_cols + word_vis > max_cols {
            // Word doesn't fit — need line break
            // Before the newline, reset so active backgrounds don't bleed to
            // end-of-line. Re-emit accumulated state on the next line.
            if line_has_ansi {
                out.push_str(RESET);
            }
            out.push('\n');
            out.push_str(indent);
            if !ansi_state.is_empty() {
                out.push_str(&ansi_state);
            }
            line_cols = indent_cols;
            line_has_ansi = !ansi_state.is_empty();

            let trimmed = word.trim_start_matches(' ');
            let trimmed_vis = visible_len(trimmed);

            // If trimmed word still too long for a full line, char-wrap it
            if trimmed_vis > max_cols.saturating_sub(indent_cols) {
                emit_long_word(
                    &mut out,
                    trimmed,
                    &mut line_cols,
                    &mut line_has_ansi,
                    &mut ansi_state,
                    max_cols,
                    indent,
                    indent_cols,
                );
            } else {
                out.push_str(trimmed);
                line_cols += trimmed_vis;
                if word_has_ansi {
                    line_has_ansi = true;
                }
            }
        } else if word_vis > max_cols {
            // Word longer than entire line — char-wrap
            emit_long_word(
                &mut out,
                &word,
                &mut line_cols,
                &mut line_has_ansi,
                &mut ansi_state,
                max_cols,
                indent,
                indent_cols,
            );
        } else {
            out.push_str(&word);
            line_cols += word_vis;
            if word_has_ansi {
                line_has_ansi = true;
            }
        }

        // Accumulate ANSI CSI sequences from this word into state.
        // A RESET sequence clears the state.
        if word_has_ansi && !word.is_empty() {
            let wb = word.as_bytes();
            let mut j = 0;
            while j < wb.len() {
                if wb[j] == b'\x1b' && j + 1 < wb.len() && wb[j + 1] == b'[' {
                    let seq_start = j;
                    j += 2;
                    while j < wb.len() && !(0x40..=0x7e).contains(&wb[j]) {
                        j += 1;
                    }
                    if j < wb.len() {
                        j += 1;
                    }
                    let seq = &word[seq_start..j];
                    // ESC[0m or ESC[m is a full reset
                    if seq == "\x1b[0m" || seq == "\x1b[m" {
                        ansi_state.clear();
                    } else {
                        ansi_state.push_str(seq);
                    }
                } else {
                    j += crate::utf8_char_len(wb[j]);
                }
            }
        }

        i = next_i;
    }

    if line_has_ansi {
        out.push_str(RESET);
    }
    out
}

/// Count the visible terminal columns occupied by `s`.
///
/// - ANSI CSI sequences (`ESC [` … final byte) contribute 0 columns.
/// - OSC sequences (`ESC ]` … BEL or `ESC \`) contribute 0 columns.
/// - Tab characters advance to the next 8-column tab stop.
/// - Multi-byte UTF-8 sequences count as 1 column each.
pub(super) fn visible_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut col = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\x1b' {
            i += 1;
            if i < bytes.len() && bytes[i] == b'[' {
                // CSI sequence
                i += 1;
                while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                    i += 1;
                }
                i += 1;
            } else if i < bytes.len() && bytes[i] == b']' {
                // OSC sequence — terminated by BEL or ESC \
                i += 1;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if b == 0x07 {
                        break;
                    }
                    if b == b'\x1b' && i < bytes.len() && bytes[i] == b'\\' {
                        i += 1;
                        break;
                    }
                }
            }
        } else if bytes[i] == b'\t' {
            col = (col / 8 + 1) * 8;
            i += 1;
        } else if bytes[i] >= 0x80 {
            let char_len = crate::utf8_char_len(bytes[i]);
            let c = s[i..].chars().next().unwrap_or('\u{FFFD}');
            col += char_width(c);
            i += char_len;
        } else {
            col += 1;
            i += 1;
        }
    }
    col
}

/// Terminal column width of a single `char`.
///
/// Returns 0 for combining marks and zero-width characters, 2 for East-Asian
/// wide / fullwidth characters and most emoji, and 1 otherwise. This is a
/// dependency-free approximation of Unicode East Asian Width (UAX #11) covering
/// the ranges that matter for CJK text and common emoji.
pub(super) fn char_width(c: char) -> usize {
    let cp = c as u32;
    // Everything below U+0300 is single-width (callers already handle ASCII, so this
    // covers Latin-1 supplement / Latin Extended): skip the range chains entirely.
    if cp < 0x0300 {
        return 1;
    }
    // Zero-width: combining marks, ZWJ/ZWNJ, ZWSP, variation selectors, BOM.
    if cp == 0x200B
        || cp == 0x200C
        || cp == 0x200D
        || cp == 0xFEFF
        || (0x0300..=0x036F).contains(&cp)
        || (0x1AB0..=0x1AFF).contains(&cp)
        || (0x1DC0..=0x1DFF).contains(&cp)
        || (0x20D0..=0x20FF).contains(&cp)
        || (0xFE00..=0xFE0F).contains(&cp)
        || (0xFE20..=0xFE2F).contains(&cp)
    {
        return 0;
    }
    // Wide / fullwidth (UAX #11 W and F). Common CJK ranges are listed first.
    if (0x4E00..=0x9FFF).contains(&cp)   // CJK Unified
        || (0xAC00..=0xD7A3).contains(&cp)   // Hangul syllables
        || (0x1100..=0x115F).contains(&cp)   // Hangul Jamo
        || (0x2E80..=0x303E).contains(&cp)   // CJK radicals, Kangxi, symbols
        || (0x3041..=0x33FF).contains(&cp)   // Hiragana..CJK compatibility
        || (0x3400..=0x4DBF).contains(&cp)   // CJK Ext A
        || (0xA000..=0xA4CF).contains(&cp)   // Yi
        || (0xF900..=0xFAFF).contains(&cp)   // CJK compatibility ideographs
        || (0xFE10..=0xFE19).contains(&cp)   // vertical forms
        || (0xFE30..=0xFE6F).contains(&cp)   // CJK compatibility forms
        || (0xFF00..=0xFF60).contains(&cp)   // fullwidth forms
        || (0xFFE0..=0xFFE6).contains(&cp)   // fullwidth signs
        || (0x1F300..=0x1FAFF).contains(&cp) // emoji / symbols & pictographs
        || (0x20000..=0x3FFFD).contains(&cp)
    // CJK Ext B+ (supplementary ideographic)
    {
        return 2;
    }
    1
}
