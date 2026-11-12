/// Compact pure-Rust QR Code Version 3-M (`29x29`) generator for short URLs (<= 42 bytes).
/// Implements ISO/IEC 18004 GF(256) Reed-Solomon ECC (poly `0x11D`, 26 ECC bytes) and mask evaluation.

const SIZE: usize = 29;
const DATA_CODEWORDS: usize = 44;
const ECC_CODEWORDS: usize = 26;

#[derive(Clone)]
pub struct QrMatrix {
    pub size: usize,
    pub modules: Vec<bool>,
}

impl QrMatrix {
    pub fn get(&self, r: usize, c: usize) -> bool {
        self.modules[r * self.size + c]
    }
}

pub fn encode_qr_v3_m(text: &str) -> Option<QrMatrix> {
    let bytes = text.as_bytes();
    if bytes.len() > 42 {
        return None;
    }

    // 1. Bit stream: Byte mode (0100) + 8-bit length + payload + terminator + 0xEC/0x11 padding
    let mut bits: Vec<u8> = Vec::with_capacity(DATA_CODEWORDS * 8);
    push_bits(&mut bits, 0b0100, 4);
    push_bits(&mut bits, bytes.len() as u32, 8);
    for &b in bytes {
        push_bits(&mut bits, b as u32, 8);
    }
    let max_bits = DATA_CODEWORDS * 8;
    let term = (max_bits - bits.len()).min(4);
    push_bits(&mut bits, 0, term);
    while bits.len() % 8 != 0 {
        bits.push(0);
    }
    let mut codewords = Vec::with_capacity(DATA_CODEWORDS + ECC_CODEWORDS);
    for chunk in bits.chunks(8) {
        let mut v = 0u8;
        for &bit in chunk {
            v = (v << 1) | bit;
        }
        codewords.push(v);
    }
    let mut pad_idx = 0usize;
    while codewords.len() < DATA_CODEWORDS {
        codewords.push(if pad_idx % 2 == 0 { 0xEC } else { 0x11 });
        pad_idx += 1;
    }

    // 2. Reed-Solomon ECC (26 codewords)
    let ecc = rs_remainder(&codewords, ECC_CODEWORDS);
    codewords.extend_from_slice(&ecc);

    // 3. Function pattern layout
    let mut modules = vec![false; SIZE * SIZE];
    let mut is_func = vec![false; SIZE * SIZE];
    place_finder(&mut modules, &mut is_func, 0, 0);
    place_finder(&mut modules, &mut is_func, 0, SIZE - 7);
    place_finder(&mut modules, &mut is_func, SIZE - 7, 0);
    place_alignment(&mut modules, &mut is_func, 22, 22);
    place_timing(&mut modules, &mut is_func);
    // Dark module at (4*3 + 9, 8) = (21, 8)
    modules[21 * SIZE + 8] = true;
    is_func[21 * SIZE + 8] = true;
    // Reserve format info strips
    for i in 0..9 {
        if i != 6 {
            is_func[8 * SIZE + i] = true;
            is_func[i * SIZE + 8] = true;
        }
    }
    for i in 0..8 {
        is_func[8 * SIZE + (SIZE - 1 - i)] = true;
    }
    for i in 0..7 {
        is_func[(SIZE - 1 - i) * SIZE + 8] = true;
    }

    // 4. Zigzag data placement
    let mut bit_idx = 0usize;
    let total_bits = codewords.len() * 8;
    let mut col = (SIZE - 1) as isize;
    let mut upward = true;
    while col > 0 {
        if col == 6 {
            col -= 1;
        }
        for step in 0..SIZE {
            let r = if upward { SIZE - 1 - step } else { step };
            for dx in [0isize, -1isize] {
                let c = (col + dx) as usize;
                let idx = r * SIZE + c;
                if !is_func[idx] {
                    let bit = if bit_idx < total_bits {
                        ((codewords[bit_idx / 8] >> (7 - (bit_idx % 8))) & 1) != 0
                    } else {
                        false
                    };
                    modules[idx] = bit;
                    bit_idx += 1;
                }
            }
        }
        col -= 2;
        upward = !upward;
    }

    // 5. Evaluate 8 masks, pick lowest penalty
    let mut best_modules = modules.clone();
    let mut best_score = u32::MAX;
    for mask in 0..8u8 {
        let mut candidate = modules.clone();
        for r in 0..SIZE {
            for c in 0..SIZE {
                if !is_func[r * SIZE + c] && mask_bit(mask, r, c) {
                    candidate[r * SIZE + c] = !candidate[r * SIZE + c];
                }
            }
        }
        write_format_info(&mut candidate, mask);
        let score = penalty_score(&candidate);
        if score < best_score {
            best_score = score;
            best_modules = candidate;
        }
    }

    Some(QrMatrix {
        size: SIZE,
        modules: best_modules,
    })
}

fn push_bits(out: &mut Vec<u8>, val: u32, count: usize) {
    for i in (0..count).rev() {
        out.push(((val >> i) & 1) as u8);
    }
}

fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut res = 0u8;
    for _ in 0..8 {
        if (b & 1) != 0 {
            res ^= a;
        }
        let hi = (a & 0x80) != 0;
        a <<= 1;
        if hi {
            a ^= 0x1D;
        }
        b >>= 1;
    }
    res
}

fn rs_remainder(data: &[u8], ecc_len: usize) -> Vec<u8> {
    let mut gen = vec![0u8; ecc_len];
    gen[ecc_len - 1] = 1;
    let mut root = 1u8;
    for _ in 0..ecc_len {
        for j in 0..ecc_len {
            gen[j] = gf_mul(gen[j], root);
            if j + 1 < ecc_len {
                gen[j] ^= gen[j + 1];
            }
        }
        root = gf_mul(root, 0x02);
    }

    let mut rem = vec![0u8; ecc_len];
    for &b in data {
        let factor = b ^ rem[0];
        rem.copy_within(1.., 0);
        rem[ecc_len - 1] = 0;
        for j in 0..ecc_len {
            rem[j] ^= gf_mul(gen[j], factor);
        }
    }
    rem
}

fn place_finder(m: &mut [bool], f: &mut [bool], r0: usize, c0: usize) {
    for r in 0..7 {
        for c in 0..7 {
            let dark = r == 0 || r == 6 || c == 0 || c == 6 || ((2..=4).contains(&r) && (2..=4).contains(&c));
            m[(r0 + r) * SIZE + (c0 + c)] = dark;
            f[(r0 + r) * SIZE + (c0 + c)] = true;
        }
    }
    // 1px white separator around 7x7 finder
    let r_start = r0.saturating_sub(1);
    let r_end = (r0 + 7).min(SIZE - 1);
    let c_start = c0.saturating_sub(1);
    let c_end = (c0 + 7).min(SIZE - 1);
    for r in r_start..=r_end {
        for c in c_start..=c_end {
            f[r * SIZE + c] = true;
        }
    }
}

fn place_alignment(m: &mut [bool], f: &mut [bool], cr: usize, cc: usize) {
    for dr in -2isize..=2 {
        for dc in -2isize..=2 {
            let r = (cr as isize + dr) as usize;
            let c = (cc as isize + dc) as usize;
            let dark = dr.abs() == 2 || dc.abs() == 2 || (dr == 0 && dc == 0);
            m[r * SIZE + c] = dark;
            f[r * SIZE + c] = true;
        }
    }
}

fn place_timing(m: &mut [bool], f: &mut [bool]) {
    for i in 8..(SIZE - 8) {
        let dark = i % 2 == 0;
        m[6 * SIZE + i] = dark;
        f[6 * SIZE + i] = true;
        m[i * SIZE + 6] = dark;
        f[i * SIZE + 6] = true;
    }
}

fn mask_bit(mask: u8, r: usize, c: usize) -> bool {
    match mask {
        0 => (r + c) % 2 == 0,
        1 => r % 2 == 0,
        2 => c % 3 == 0,
        3 => (r + c) % 3 == 0,
        4 => ((r / 2) + (c / 3)) % 2 == 0,
        5 => ((r * c) % 2) + ((r * c) % 3) == 0,
        6 => (((r * c) % 2) + ((r * c) % 3)) % 2 == 0,
        _ => (((r + c) % 2) + ((r * c) % 3)) % 2 == 0,
    }
}

fn write_format_info(m: &mut [bool], mask: u8) {
    // Error correction level M = 0b00, 5 data bits + 10 BCH(15, 5) ECC bits
    let data = (0b00u32 << 3) | (mask as u32 & 0b111);
    let mut rem = data << 10;
    for shift in (0..5).rev() {
        if ((rem >> (shift + 10)) & 1) != 0 {
            rem ^= 0x537 << shift;
        }
    }
    let bits = ((data << 10) | rem) ^ 0x5412;

    for i in 0..15 {
        let dark = ((bits >> i) & 1) != 0;
        // Copy 1 (top-left): LSB at (0, 8) -> (8, 8) -> MSB at (8, 0)
        if i < 6 {
            m[i * SIZE + 8] = dark;
        } else if i == 6 {
            m[7 * SIZE + 8] = dark;
        } else if i == 7 {
            m[8 * SIZE + 8] = dark;
        } else if i == 8 {
            m[8 * SIZE + 7] = dark;
        } else {
            m[8 * SIZE + (14 - i)] = dark;
        }

        // Copy 2 (top-right + bottom-left): LSB at (8, SIZE-1) -> MSB at (SIZE-1, 8); leaves (SIZE-8, 8) dark module intact
        if i < 8 {
            m[8 * SIZE + (SIZE - 1 - i)] = dark;
        } else {
            m[(SIZE - 15 + i) * SIZE + 8] = dark;
        }
    }
}

fn penalty_score(m: &[bool]) -> u32 {
    let mut score = 0u32;
    for r in 0..SIZE {
        let mut run_col = m[r * SIZE];
        let mut run_len = 1u32;
        for c in 1..SIZE {
            let v = m[r * SIZE + c];
            if v == run_col {
                run_len += 1;
            } else {
                if run_len >= 5 {
                    score += 3 + (run_len - 5);
                }
                run_col = v;
                run_len = 1;
            }
        }
        if run_len >= 5 {
            score += 3 + (run_len - 5);
        }
    }
    for c in 0..SIZE {
        let mut run_col = m[c];
        let mut run_len = 1u32;
        for r in 1..SIZE {
            let v = m[r * SIZE + c];
            if v == run_col {
                run_len += 1;
            } else {
                if run_len >= 5 {
                    score += 3 + (run_len - 5);
                }
                run_col = v;
                run_len = 1;
            }
        }
        if run_len >= 5 {
            score += 3 + (run_len - 5);
        }
    }
    score
}

/// Resolves the PC's active LAN IPv4 address for mobile QR pairing (matching C++ `GetLocalIPv4`).
pub fn detect_lan_ipv4() -> String {
    if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if sock.connect("8.8.8.8:53").is_ok() {
            if let Ok(addr) = sock.local_addr() {
                let ip = addr.ip().to_string();
                if ip != "0.0.0.0" && !ip.starts_with("127.") {
                    return ip;
                }
            }
        }
    }
    "127.0.0.1".to_string()
}

#[cfg(test)]
mod tests {
    use super::encode_qr_v3_m;

    #[test]
    fn encodes_lan_url_into_29x29_qr_with_finders() {
        let qr = encode_qr_v3_m("http://192.168.1.25:8766").unwrap();
        assert_eq!(qr.size, 29);
        assert_eq!(qr.modules.len(), 29 * 29);
        // Top-left finder corners must be dark
        assert!(qr.get(0, 0));
        assert!(qr.get(0, 6));
        assert!(qr.get(6, 0));
        // Dark module (21, 8)
        assert!(qr.get(21, 8));
    }
}
