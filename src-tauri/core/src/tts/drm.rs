//! Sec-MS-GEC v2: the token every request to the Edge speech service carries (port of edge-tts
//! 7.3 drm.py): "<CS64 hex>-<SHA-256 variant hex>" of the 5-minute Windows file time and the
//! public trusted-client token.

use super::drm_sha;

pub const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const WIN_EPOCH: u64 = 11_644_473_600;

const CS64: [(u32, u32); 14] = [
    (0xE82E91B1, 0xB32700D0), (0x44DE24C6, 0x88DFF51D), (0x3BEB65BC, 0x29514038), (0xC89C5933, 0xB328CBDF),
    (0x8D1EC211, 0xD405B4D5), (0x314EF5D5, 0x26E55F3C), (0xE92491B1, 0x607801C6), (0xF0A5B9D5, 0x82B2C1FB),
    (0x06064CF3, 0xFA51A3DA), (0xDA79FA60, 0x207CC464), (0xBED20560, 0x3C3C74F3), (0x89F9C9F8, 0xD2D14B61),
    (0x35FA38C6, 0xBB04A2C1), (0xCB49E3C6, 0xC5626167),
];
const CS64_FACTOR: u32 = 0x82893933;
const CS64_ADJUSTMENT0: u32 = 0xF2F90B1A;
const CS64_ADJUSTMENT1: u32 = 0x0B1A0000;

fn rol16(v: u32) -> u32 {
    v.rotate_left(16)
}

/// Little-endian 32-bit words, zero-padded to an even count.
fn words_le_even(data: &[u8]) -> Vec<u32> {
    let mut padded = data.to_vec();
    while padded.len() % 4 != 0 {
        padded.push(0);
    }
    let mut words: Vec<u32> =
        padded.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
    if words.len() % 2 == 1 {
        words.push(0);
    }
    words
}

fn cs64(data: &[u8], digest: &[u8; 32]) -> (u32, u32) {
    let key_a = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) | 1;
    let key_b = u32::from_le_bytes([digest[4], digest[5], digest[6], digest[7]]) | 1;
    let factors = [key_a.wrapping_mul(CS64_FACTOR), key_b.wrapping_mul(CS64_FACTOR)];
    let mut terms: Vec<u32> = CS64.iter().enumerate().map(|(i, (c, _))| c.wrapping_mul(factors[i % 2])).collect();
    let mut offsets: Vec<u32> = CS64.iter().map(|(_, o)| *o).collect();
    let mut input = vec![0u8];
    input.extend_from_slice(data);
    for (i, word) in words_le_even(&input).into_iter().enumerate() {
        let even = i % 2 == 0;
        let mut term = word.wrapping_mul(if even { key_a } else { key_b });
        let mut offset = rol16(word).wrapping_mul(if even { 0xEF0569FB } else { 0xC31713DB });
        if i == 0 {
            term = term.wrapping_add(CS64_ADJUSTMENT0.wrapping_mul(factors[0]));
            offset = offset.wrapping_add(CS64_ADJUSTMENT1);
        }
        terms.push(term);
        offsets.push(offset);
    }
    let (mut swap_acc, mut swap_last, mut state, mut acc) = (0u32, 0u32, 0u32, 0u32);
    for i in (0..terms.len()).step_by(2) {
        let (x, y) = (terms[i], terms[i + 1]);
        let q = rol16(offsets[i].wrapping_add(x)).wrapping_mul(0x76F50F34);
        swap_acc = swap_acc.wrapping_add(rol16(q).wrapping_mul(0xC3EFEA97).wrapping_add(q));
        let q = rol16(offsets[i + 1].wrapping_add(y)).wrapping_mul(0x1881CE3C);
        swap_last = rol16(q).wrapping_mul(0x35BD1EC9).wrapping_add(q);
        swap_acc = swap_acc.wrapping_add(swap_last);
        state = state.wrapping_mul(key_a).wrapping_add(x);
        for m in [0xCF98B111u32, 0x87085B9F, 0x12CEB96D, 0x257E1D83] {
            state = rol16(state).wrapping_mul(m);
        }
        acc = acc.wrapping_add(state);
        let mut t = state.wrapping_mul(key_b).wrapping_add(y);
        t = rol16(t).wrapping_mul(0xA27416F5);
        t = rol16(t).wrapping_mul(0xD38396FF);
        state = rol16(rol16(t).wrapping_mul(0x7C932B89)).wrapping_mul(0xBFA49F69);
        acc = acc.wrapping_add(state);
    }
    (swap_last ^ state, swap_acc ^ acc)
}

/// The token for a Unix time (seconds).
pub fn sec_ms_gec(unix_seconds: f64) -> String {
    let secs = unix_seconds.max(0.0).floor() as u64 + WIN_EPOCH;
    let ticks = u128::from(secs - secs % 300) * 10_000_000;
    let data = format!("{ticks}{TRUSTED_CLIENT_TOKEN}");
    let digest = drm_sha::digest(data.as_bytes());
    let (lo, hi) = cs64(data.as_bytes(), &digest);
    let mut cs = Vec::with_capacity(8);
    cs.extend_from_slice(&lo.to_le_bytes());
    cs.extend_from_slice(&hi.to_le_bytes());
    format!("{}-{}", hex::encode_upper(cs), hex::encode_upper(digest))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Values printed by edge-tts 7.3.1 (generate_sec_ms_gec_v2) for the same times.
    #[test]
    fn matches_edge_tts() {
        assert_eq!(
            sec_ms_gec(1_700_000_000.0),
            "2CD23A2CA0B2595A-F3C846C370E05F9CC83ACC5F995C5F5D87A4D0F14CE8936A0F902E64099E38A2"
        );
        assert_eq!(
            sec_ms_gec(1_762_600_000.7),
            "32867DDE941117D8-371C0B0740F6E768F46C1308F20143CB3199EDE1C27EDE417D84DF918DF601FC"
        );
        assert_eq!(
            sec_ms_gec(1_800_000_123.0),
            "99E54519B490BE5C-E162DB0977FBDCB9B86DD8148BF1BCDC3C0303176CE39852A42B415935C20654"
        );
    }

    #[test]
    fn digest_matches_edge_tts() {
        let d = drm_sha::digest(b"13377734400000000006A5AA1D4EAFF4E9FB37E23D68491D6F4");
        assert_eq!(hex::encode(d), "d3fbbaddad1e147e5bc331e869144e7ed4588eb1bec11c04e3f87ed078efbf17");
    }
}
