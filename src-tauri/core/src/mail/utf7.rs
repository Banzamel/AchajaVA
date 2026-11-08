//! IMAP folder names are "modified UTF-7" (RFC 3501 5.1.3): "Wysłane" is "Wys&AUI-ane".

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+,";

fn flush(out: &mut String, pending: &mut Vec<u16>) {
    if pending.is_empty() {
        return;
    }
    let bytes: Vec<u8> = pending.iter().flat_map(|u| u.to_be_bytes()).collect();
    out.push('&');
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, b)| acc | (u32::from(*b) << (16 - 8 * i)));
        let count = chunk.len() + 1;
        for i in 0..count {
            out.push(B64[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out.push('-');
    pending.clear();
}

pub fn encode(name: &str) -> String {
    let mut out = String::new();
    let mut pending: Vec<u16> = Vec::new();
    for c in name.chars() {
        if (' '..='~').contains(&c) {
            flush(&mut out, &mut pending);
            if c == '&' {
                out.push_str("&-");
            } else {
                out.push(c);
            }
        } else {
            pending.extend(c.encode_utf16(&mut [0; 2]).iter());
        }
    }
    flush(&mut out, &mut pending);
    out
}

pub fn decode(name: &str) -> String {
    let mut out = String::new();
    let mut rest = name;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after.find('-').unwrap_or(after.len());
        let chunk = &after[..end];
        if chunk.is_empty() {
            out.push('&');
        } else {
            let mut bits = 0u32;
            let mut nbits = 0;
            let mut bytes = Vec::new();
            for c in chunk.bytes() {
                let Some(v) = B64.iter().position(|b| *b == c) else { continue };
                bits = (bits << 6) | v as u32;
                nbits += 6;
                if nbits >= 8 {
                    nbits -= 8;
                    bytes.push((bits >> nbits) as u8);
                }
            }
            let units: Vec<u16> = bytes.chunks(2).filter(|c| c.len() == 2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            out.push_str(&String::from_utf16_lossy(&units));
        }
        rest = after.get(end + 1..).unwrap_or("");
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_polish_names() {
        assert_eq!(encode("Wysłane"), "Wys&AUI-ane");
        assert_eq!(decode("Wys&AUI-ane"), "Wysłane");
        assert_eq!(encode("A&B"), "A&-B");
        assert_eq!(decode("A&-B"), "A&B");
        assert_eq!(decode(&encode("Kosz ąęść")), "Kosz ąęść");
    }
}
