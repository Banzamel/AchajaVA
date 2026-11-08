//! The SHA-256 variant of the Edge speech service's Sec-MS-GEC v2 token (port of edge-tts 7.3
//! drm.py): ordinary SHA-256 rounds, but the first block starts from a fixed mid-state with an
//! adjusted message schedule. Checked against edge-tts' own output in the tests of drm.rs.

const INITIAL: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];
const STATE: [u32; 8] = [
    0xFD9FBEA2, 0x91243CE7, 0xBE499094, 0xCD2E1E0A, 0x6C006F49, 0x04BD35AB, 0xFEB0A7E4, 0x03EFFBD2,
];
const SCHEDULE: [u32; 14] = [
    0xEB24BF9E, 0x1558691F, 0xA0C09EE2, 0x0CFA32BA, 0xA622EE8C, 0xD1DD9491, 0xC0DD25A0, 0xF9E70AB6,
    0xF0C31573, 0x7058CE4E, 0x7188E606, 0x00E5AB33, 0xD258B060, 0x72766564,
];
const K: [u32; 64] = [
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5, 0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3, 0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC, 0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7, 0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13, 0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3, 0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5, 0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208, 0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
];
const ADJUSTMENT: u32 = 0xC9DC06A7;
const SIGMA0_ADJUSTMENT: u32 = 0x059C0B80;
const SIGMA1_ADJUSTMENT: u32 = 0x000B92C0;

fn sigma0(v: u32) -> u32 {
    v.rotate_right(7) ^ v.rotate_right(18) ^ (v >> 3)
}

fn sigma1(v: u32) -> u32 {
    v.rotate_right(17) ^ v.rotate_right(19) ^ (v >> 10)
}

fn expand(words: &mut Vec<u32>, first: usize) {
    for i in first..64 {
        let w = sigma1(words[i - 2])
            .wrapping_add(words[i - 7])
            .wrapping_add(sigma0(words[i - 15]))
            .wrapping_add(words[i - 16]);
        words.push(w);
    }
}

/// SHA-256 rounds from `first` to 63; the mid-state block (first == 14) uses K[14] = ADJUSTMENT.
fn rounds(state: [u32; 8], words: &[u32], first: usize) -> [u32; 8] {
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
    for i in first..64 {
        let k = if first == 14 && i == 14 { ADJUSTMENT } else { K[i] };
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = g ^ (e & (f ^ g));
        let t1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(k).wrapping_add(words[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) | (c & (a | b));
        let t2 = s0.wrapping_add(maj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    [a, b, c, d, e, f, g, h]
}

fn add(x: [u32; 8], y: [u32; 8]) -> [u32; 8] {
    let mut out = [0u32; 8];
    for i in 0..8 {
        out[i] = x[i].wrapping_add(y[i]);
    }
    out
}

fn be(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0u32, |acc, b| (acc << 8) | u32::from(*b))
}

/// The state after the first block, built from the first seven bytes of the token data.
fn init(data: &[u8]) -> [u32; 8] {
    let mut w: Vec<u32> = vec![0; 14];
    w.push(be(&data[..3]));
    w.push(be(&data[3..7]));
    for i in 16..30 {
        let mut v = SCHEDULE[i - 16];
        if i - 2 >= 14 {
            let s = sigma1(w[i - 2]);
            v = v.wrapping_add(if i == 16 { s ^ SIGMA1_ADJUSTMENT } else { s });
        }
        if i - 7 >= 14 {
            v = v.wrapping_add(w[i - 7]);
        }
        if i >= 29 {
            let s = sigma0(w[i - 15]);
            v = v.wrapping_add(if i == 29 { s ^ SIGMA0_ADJUSTMENT } else { s });
        }
        w.push(v);
    }
    let last = sigma1(w[28])
        .wrapping_add(w[23])
        .wrapping_add(sigma0(w[15]))
        .wrapping_add(w[14])
        .wrapping_add(ADJUSTMENT)
        .wrapping_sub(K[14]);
    w.push(last);
    expand(&mut w, 31);
    add(INITIAL, rounds(STATE, &w, 14))
}

pub fn digest(data: &[u8]) -> [u8; 32] {
    let length = 57 + data.len();
    let mut padded = data[7..].to_vec();
    padded.push(0x80);
    let zeros = (55i64 - length as i64).rem_euclid(64) as usize;
    padded.extend(std::iter::repeat(0u8).take(zeros));
    padded.extend(((length as u64) * 8).to_be_bytes());
    let mut state = init(&data[..7]);
    for block in padded.chunks(64) {
        let mut w: Vec<u32> = block.chunks(4).map(be).collect();
        expand(&mut w, 16);
        state = add(state, rounds(state, &w, 0));
    }
    let mut out = [0u8; 32];
    for (i, v) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}
