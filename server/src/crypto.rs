const SEED: u64 = 0x9E3779B97F4A7C15;
const BLOCK: usize = 16;

fn keystream(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut x = SEED;
    while out.len() < len {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        out.extend_from_slice(&x.to_le_bytes());
    }
    out.truncate(len);
    out
}

// Shift amounts are derived purely from the block index (not from the data),
// so rotation is trivially invertible with rotate_right using the same shift.
fn block_shift(block_index: usize, n: usize) -> usize {
    if n < 2 {
        return 0;
    }
    ((block_index.wrapping_mul(2654435761).wrapping_add(11)) % n).max(1) % n
}

pub fn obfuscate(data: &[u8]) -> Vec<u8> {
    let mut buf = data.to_vec();
    for (i, chunk) in buf.chunks_mut(BLOCK).enumerate() {
        let shift = block_shift(i, chunk.len());
        chunk.rotate_left(shift);
    }
    let ks = keystream(buf.len());
    for (b, k) in buf.iter_mut().zip(ks.iter()) {
        *b ^= k;
    }
    buf
}

pub fn deobfuscate(data: &[u8]) -> Vec<u8> {
    let mut buf = data.to_vec();
    let ks = keystream(buf.len());
    for (b, k) in buf.iter_mut().zip(ks.iter()) {
        *b ^= k;
    }
    for (i, chunk) in buf.chunks_mut(BLOCK).enumerate() {
        let shift = block_shift(i, chunk.len());
        chunk.rotate_right(shift);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        let data: &[u8] = b"";
        assert_eq!(deobfuscate(&obfuscate(data)), data);
    }

    #[test]
    fn roundtrip_short() {
        let data = b"hi";
        assert_eq!(deobfuscate(&obfuscate(data)), data);
    }

    #[test]
    fn roundtrip_various_lengths() {
        for len in 0..200 {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 5) as u8).collect();
            let enc = obfuscate(&data);
            let dec = deobfuscate(&enc);
            assert_eq!(dec, data, "failed at len {}", len);
        }
    }

    #[test]
    fn output_differs_from_input() {
        let data = vec![0u8; 64];
        let enc = obfuscate(&data);
        assert_ne!(enc, data);
    }
}
