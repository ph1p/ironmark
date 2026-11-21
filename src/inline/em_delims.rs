use super::*;

#[derive(Clone, Copy)]
pub(super) struct EmDelim {
    pub(super) orig_start: u32,
    pub(super) orig_end: u32,
    pub(super) cur_start: u32,
    pub(super) cur_end: u32,
    pub(super) marker: u8,
    pub(super) can_open: bool,
    pub(super) can_close: bool,
    pub(super) active: bool,
    pub(super) open_em: [u8; 4],
    pub(super) open_em_len: u8,
    pub(super) close_em: [u8; 4],
    pub(super) close_em_len: u8,
}

#[inline(never)]
pub(super) fn process_em_delims(delims: &mut [EmDelim]) {
    let mut openers_bottom = [0usize; 6];
    let mut closer_idx = 0usize;
    while closer_idx < delims.len() {
        if !delims[closer_idx].active
            || !delims[closer_idx].can_close
            || delims[closer_idx].cur_end == delims[closer_idx].cur_start
        {
            closer_idx += 1;
            continue;
        }
        let cmarker = delims[closer_idx].marker;
        let ccount = (delims[closer_idx].cur_end - delims[closer_idx].cur_start) as u16;
        let bottom = openers_bottom[em_openers_index(cmarker, ccount)];

        let mut found = false;
        let mut oi = closer_idx;
        while oi > bottom {
            oi -= 1;
            if !delims[oi].active || delims[oi].marker != cmarker || !delims[oi].can_open {
                continue;
            }
            let ocount = (delims[oi].cur_end - delims[oi].cur_start) as u16;
            if ocount == 0 {
                continue;
            }
            if (delims[oi].can_close || delims[closer_idx].can_open)
                && (ocount + ccount).is_multiple_of(3)
                && (!ocount.is_multiple_of(3) || !ccount.is_multiple_of(3))
            {
                continue;
            }
            let use_count: u16 = if ocount >= 2 && ccount >= 2 { 2 } else { 1 };
            let tag = use_count as u8;

            delims[oi].cur_end -= use_count as u32;
            let idx = delims[oi].open_em_len as usize;
            if idx < 4 {
                delims[oi].open_em[idx] = tag;
                delims[oi].open_em_len += 1;
            }

            delims[closer_idx].cur_start += use_count as u32;
            let idx = delims[closer_idx].close_em_len as usize;
            if idx < 4 {
                delims[closer_idx].close_em[idx] = tag;
                delims[closer_idx].close_em_len += 1;
            }

            for delim in delims.iter_mut().take(closer_idx).skip(oi + 1) {
                delim.active = false;
            }

            if delims[oi].cur_end == delims[oi].cur_start {
                delims[oi].active = false;
            }
            if delims[closer_idx].cur_end == delims[closer_idx].cur_start {
                delims[closer_idx].active = false;
            }

            found = true;
            break;
        }
        if !found {
            openers_bottom[em_openers_index(cmarker, ccount)] = closer_idx;
            if !delims[closer_idx].can_open {
                delims[closer_idx].active = false;
            }
            closer_idx += 1;
        }
    }
}

#[inline(always)]
pub(super) fn em_openers_index(marker: u8, count: u16) -> usize {
    match marker {
        b'*' => (count % 3) as usize,
        b'_' => 3 + (count % 3) as usize,
        _ => 0,
    }
}

#[inline(always)]
pub(super) fn render_em_delim(out: &mut String, d: &EmDelim) {
    for &tag in &d.close_em[..d.close_em_len as usize] {
        out.push_str(if tag == 2 { "</strong>" } else { "</em>" });
    }
    if d.cur_start < d.cur_end {
        let marker = d.marker as char;
        for _ in 0..(d.cur_end - d.cur_start) {
            out.push(marker);
        }
    }
    for j in (0..d.open_em_len as usize).rev() {
        out.push_str(if d.open_em[j] == 2 {
            "<strong>"
        } else {
            "<em>"
        });
    }
}

#[inline]
pub(super) fn scan_em_delims(raw: &str, bytes: &[u8], skip_escapes: bool, buf: &mut Vec<EmDelim>) {
    buf.clear();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        let next = if skip_escapes {
            memchr::memchr3(b'*', b'_', b'\\', &bytes[i..])
        } else {
            memchr::memchr2(b'*', b'_', &bytes[i..])
        };
        let Some(off) = next else { break };
        i += off;
        let b = bytes[i];
        if b == b'\\' {
            // skip_escapes must be true if we got here
            if i + 1 < len && crate::is_ascii_punctuation(bytes[i + 1]) {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        let run_start = i;
        i += 1;
        while i < len && bytes[i] == b {
            i += 1;
        }
        let before = if run_start > 0 {
            char_before(raw, run_start)
        } else {
            ' '
        };
        let after = if i < len { char_at(raw, i) } else { ' ' };
        let (can_open, can_close) = flanking(b, before, after);
        buf.push(EmDelim {
            orig_start: run_start as u32,
            orig_end: i as u32,
            cur_start: run_start as u32,
            cur_end: i as u32,
            marker: b,
            can_open,
            can_close,
            active: true,
            open_em: [0; 4],
            open_em_len: 0,
            close_em: [0; 4],
            close_em_len: 0,
        });
    }
}

#[inline]
pub(super) fn emit_emphasis_only(
    out: &mut String,
    raw: &str,
    bytes: &[u8],
    em_buf: &mut Vec<EmDelim>,
) {
    scan_em_delims(raw, bytes, false, em_buf);
    if em_buf.is_empty() {
        escape_html_into(out, raw);
        return;
    }
    process_em_delims(em_buf);

    let mut text_pos = 0usize;
    for d in em_buf.iter() {
        if text_pos < d.orig_start as usize {
            escape_html_into(out, &raw[text_pos..d.orig_start as usize]);
        }
        render_em_delim(out, d);
        text_pos = d.orig_end as usize;
    }
    if text_pos < bytes.len() {
        escape_html_into(out, &raw[text_pos..]);
    }
}

pub(super) fn emit_breaks_and_emphasis(
    out: &mut String,
    raw: &str,
    bytes: &[u8],
    opts: &ParseOptions,
    em_buf: &mut Vec<EmDelim>,
) {
    scan_em_delims(raw, bytes, true, em_buf);
    if !em_buf.is_empty() {
        process_em_delims(em_buf);
    }

    let len = bytes.len();
    let mut text_pos = 0usize;
    let mut di = 0usize;

    while text_pos < len {
        let next_delim_pos = if di < em_buf.len() {
            em_buf[di].orig_start as usize
        } else {
            len
        };
        emit_text_with_breaks(out, raw, bytes, &mut text_pos, next_delim_pos, opts);

        if di < em_buf.len() && text_pos == em_buf[di].orig_start as usize {
            render_em_delim(out, &em_buf[di]);
            text_pos = em_buf[di].orig_end as usize;
            di += 1;
        }
    }
}

#[inline]
pub(super) fn emit_text_with_breaks(
    out: &mut String,
    raw: &str,
    bytes: &[u8],
    text_pos: &mut usize,
    end: usize,
    opts: &ParseOptions,
) {
    let mut seg_start = *text_pos;
    let mut i = *text_pos;
    while i < end {
        let off = memchr::memchr2(b'\n', b'\\', &bytes[i..end]);
        match off {
            None => break,
            Some(off) => i += off,
        }
        let b = bytes[i];
        if b == b'\n' {
            let mut text_end = i;
            let is_hard = (i >= seg_start + 2 && bytes[i - 1] == b' ' && bytes[i - 2] == b' ')
                || (i > seg_start && bytes[i - 1] == b'\\');
            if is_hard && i > seg_start && bytes[i - 1] == b'\\' {
                text_end = i - 1;
            } else {
                while text_end > seg_start && bytes[text_end - 1] == b' ' {
                    text_end -= 1;
                }
            }
            if seg_start < text_end {
                escape_html_into(out, &raw[seg_start..text_end]);
            }
            if is_hard || opts.hard_breaks {
                out.push_str("<br />\n");
            } else {
                out.push('\n');
            }
            i += 1;
            seg_start = i;
        } else {
            // b == b'\\'
            if i + 1 < end {
                let next = bytes[i + 1];
                if next == b'\n' {
                    i += 1;
                    continue;
                }
                if crate::is_ascii_punctuation(next) {
                    if seg_start < i {
                        escape_html_into(out, &raw[seg_start..i]);
                    }
                    match next {
                        b'&' => out.push_str("&amp;"),
                        b'<' => out.push_str("&lt;"),
                        b'>' => out.push_str("&gt;"),
                        b'"' => out.push_str("&quot;"),
                        _ => out.push_str(&raw[i + 1..i + 2]),
                    }
                    i += 2;
                    seg_start = i;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }
    if seg_start < end {
        escape_html_into(out, &raw[seg_start..end]);
    }
    *text_pos = end;
}
