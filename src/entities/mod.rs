mod data;

pub(crate) use data::ENTITIES;

pub(crate) static MAX_ENTITY_LEN: [u8; 128] = {
    let mut t = [0u8; 128];
    t[b'A' as usize] = 13;
    t[b'B' as usize] = 10;
    t[b'C' as usize] = 31;
    t[b'D' as usize] = 24;
    t[b'E' as usize] = 20;
    t[b'F' as usize] = 21;
    t[b'G' as usize] = 17;
    t[b'H' as usize] = 14;
    t[b'I' as usize] = 14;
    t[b'J' as usize] = 6;
    t[b'K' as usize] = 6;
    t[b'L' as usize] = 19;
    t[b'M' as usize] = 11;
    t[b'N' as usize] = 23;
    t[b'O' as usize] = 20;
    t[b'P' as usize] = 18;
    t[b'Q' as usize] = 4;
    t[b'R' as usize] = 20;
    t[b'S' as usize] = 19;
    t[b'T' as usize] = 14;
    t[b'U' as usize] = 16;
    t[b'V' as usize] = 17;
    t[b'W' as usize] = 5;
    t[b'X' as usize] = 4;
    t[b'Y' as usize] = 6;
    t[b'Z' as usize] = 14;
    t[b'a' as usize] = 8;
    t[b'b' as usize] = 18;
    t[b'c' as usize] = 16;
    t[b'd' as usize] = 16;
    t[b'e' as usize] = 12;
    t[b'f' as usize] = 13;
    t[b'g' as usize] = 10;
    t[b'h' as usize] = 14;
    t[b'i' as usize] = 8;
    t[b'j' as usize] = 6;
    t[b'k' as usize] = 6;
    t[b'l' as usize] = 19;
    t[b'm' as usize] = 13;
    t[b'n' as usize] = 16;
    t[b'o' as usize] = 8;
    t[b'p' as usize] = 11;
    t[b'q' as usize] = 11;
    t[b'r' as usize] = 17;
    t[b's' as usize] = 15;
    t[b't' as usize] = 17;
    t[b'u' as usize] = 14;
    t[b'v' as usize] = 16;
    t[b'w' as usize] = 6;
    t[b'x' as usize] = 6;
    t[b'y' as usize] = 6;
    t[b'z' as usize] = 7;
    t
};

static ENTITY_FIRST_CHAR: [(u16, u16); 128] = {
    let mut t = [(0u16, 0u16); 128];
    t[b'A' as usize] = (0, 18);
    t[b'B' as usize] = (19, 30);
    t[b'C' as usize] = (31, 64);
    t[b'D' as usize] = (65, 118);
    t[b'E' as usize] = (119, 143);
    t[b'F' as usize] = (144, 151);
    t[b'G' as usize] = (152, 172);
    t[b'H' as usize] = (173, 184);
    t[b'I' as usize] = (185, 209);
    t[b'J' as usize] = (210, 216);
    t[b'K' as usize] = (217, 224);
    t[b'L' as usize] = (225, 283);
    t[b'M' as usize] = (284, 292);
    t[b'N' as usize] = (293, 363);
    t[b'O' as usize] = (364, 386);
    t[b'P' as usize] = (387, 405);
    t[b'Q' as usize] = (406, 409);
    t[b'R' as usize] = (410, 453);
    t[b'S' as usize] = (454, 493);
    t[b'T' as usize] = (494, 515);
    t[b'U' as usize] = (516, 551);
    t[b'V' as usize] = (552, 568);
    t[b'W' as usize] = (569, 573);
    t[b'X' as usize] = (574, 577);
    t[b'Y' as usize] = (578, 587);
    t[b'Z' as usize] = (588, 597);
    t[b'a' as usize] = (598, 657);
    t[b'b' as usize] = (658, 772);
    t[b'c' as usize] = (773, 866);
    t[b'd' as usize] = (867, 930);
    t[b'e' as usize] = (931, 992);
    t[b'f' as usize] = (993, 1028);
    t[b'g' as usize] = (1029, 1087);
    t[b'h' as usize] = (1088, 1115);
    t[b'i' as usize] = (1116, 1165);
    t[b'j' as usize] = (1166, 1173);
    t[b'k' as usize] = (1174, 1183);
    t[b'l' as usize] = (1184, 1335);
    t[b'm' as usize] = (1336, 1372);
    t[b'n' as usize] = (1373, 1537);
    t[b'o' as usize] = (1538, 1590);
    t[b'p' as usize] = (1591, 1656);
    t[b'q' as usize] = (1657, 1666);
    t[b'r' as usize] = (1667, 1768);
    t[b's' as usize] = (1769, 1920);
    t[b't' as usize] = (1921, 1976);
    t[b'u' as usize] = (1977, 2023);
    t[b'v' as usize] = (2024, 2065);
    t[b'w' as usize] = (2066, 2076);
    t[b'x' as usize] = (2077, 2100);
    t[b'y' as usize] = (2101, 2111);
    t[b'z' as usize] = (2112, 2124);
    t
};

#[inline]
pub(crate) fn lookup_entity_codepoints(name: &str) -> Option<(u32, u32)> {
    let bytes = name.as_bytes();
    let first = bytes[0];

    match (first, bytes.len()) {
        (b'a', 3) if bytes[1] == b'm' && bytes[2] == b'p' => return Some((0x26, 0)),
        (b'l', 2) if bytes[1] == b't' => return Some((0x3C, 0)),
        (b'g', 2) if bytes[1] == b't' => return Some((0x3E, 0)),
        (b'q', 4) if bytes == b"quot" => return Some((0x22, 0)),
        (b'n', 4) if bytes == b"nbsp" => return Some((0xA0, 0)),
        (b'c', 4) if bytes == b"copy" => return Some((0xA9, 0)),
        _ => {}
    }
    if first >= 128 {
        return None;
    }
    let (start, end) = ENTITY_FIRST_CHAR[first as usize];
    if start == 0 && end == 0 && first != b'A' {
        return None;
    }
    let slice = &ENTITIES[start as usize..=end as usize];
    match slice.binary_search_by(|(n, _, _)| n.cmp(&name)) {
        Ok(i) => Some((slice[i].1, slice[i].2)),
        Err(_) => None,
    }
}

#[inline(always)]
fn push_codepoints(out: &mut String, cp1: u32, cp2: u32) {
    if let Some(c) = char::from_u32(cp1) {
        out.push(c);
    }
    if cp2 != 0 {
        if let Some(c) = char::from_u32(cp2) {
            out.push(c);
        }
    }
}

#[inline]
pub(crate) fn lookup_entity_into(name: &str, out: &mut String) -> bool {
    if let Some((cp1, cp2)) = lookup_entity_codepoints(name) {
        push_codepoints(out, cp1, cp2);
        true
    } else {
        false
    }
}

pub(crate) fn resolve_numeric_ref_into(value: &str, hex: bool, out: &mut String) -> bool {
    let cp = if hex {
        match u32::from_str_radix(value, 16) {
            Ok(v) => v,
            Err(_) => return false,
        }
    } else {
        match value.parse::<u32>() {
            Ok(v) => v,
            Err(_) => return false,
        }
    };

    let cp = if cp == 0 { 0xFFFD } else { cp };
    let c = char::from_u32(cp).unwrap_or('\u{FFFD}');
    out.push(c);
    true
}
