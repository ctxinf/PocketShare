const SEED: u64 = 0x9E3779B97F4A7C15;
pub const BLOCK: usize = 16;

// splitmix64: given any 64-bit index, produces an unrelated pseudorandom
// 64-bit word in O(1), independent of every other index. This lets us
// generate the keystream at an arbitrary absolute byte offset directly
// (needed for HTTP Range requests on large files) instead of having to
// replay a sequential stream cipher from position 0 every time.
fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

// `offset` must be a multiple of 8 (word size). Callers only ever request
// keystream starting at a BLOCK (16-byte) boundary, and 16 is a multiple
// of 8, so this always holds.
fn keystream_at(offset: usize, len: usize) -> Vec<u8> {
    debug_assert_eq!(offset % 8, 0);
    let mut out = Vec::with_capacity(len);
    let mut word_index = (offset / 8) as u64;
    while out.len() < len {
        out.extend_from_slice(&splitmix64(SEED ^ word_index).to_le_bytes());
        word_index = word_index.wrapping_add(1);
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
    let ks = keystream_at(0, buf.len());
    for (b, k) in buf.iter_mut().zip(ks.iter()) {
        *b ^= k;
    }
    buf
}

pub fn deobfuscate(data: &[u8]) -> Vec<u8> {
    deobfuscate_range(data, 0)
}

/// Decrypts `data`, which must be the ciphertext bytes starting at absolute
/// offset `start_offset` within the full obfuscated stream. `start_offset`
/// must be a multiple of BLOCK so block boundaries line up with the
/// original obfuscation pass (callers align Range reads to BLOCK for this
/// reason). The final block of the *whole file* may be shorter than BLOCK;
/// as long as `data` actually contains that trailing partial block (i.e.
/// the caller didn't stop short of EOF for no reason), the shift is
/// recomputed with the correct (possibly shorter) chunk length and decodes
/// correctly.
pub fn deobfuscate_range(data: &[u8], start_offset: usize) -> Vec<u8> {
    debug_assert_eq!(start_offset % BLOCK, 0);
    let mut buf = data.to_vec();
    let ks = keystream_at(start_offset, buf.len());
    for (b, k) in buf.iter_mut().zip(ks.iter()) {
        *b ^= k;
    }
    let block_index_start = start_offset / BLOCK;
    for (bi, chunk) in buf.chunks_mut(BLOCK).enumerate() {
        let shift = block_shift(block_index_start + bi, chunk.len());
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

    #[test]
    fn range_decrypt_matches_full_decrypt_slice() {
        let data: Vec<u8> = (0..5000u32).map(|i| (i * 91 + 13) as u8).collect();
        let enc = obfuscate(&data);

        // Try a bunch of BLOCK-aligned windows across the buffer.
        let mut start = 0usize;
        while start < enc.len() {
            let end = (start + 37).min(enc.len()); // arbitrary, non-block-multiple window
            let block_start = (start / BLOCK) * BLOCK;
            let block_end = (((end + BLOCK - 1) / BLOCK) * BLOCK).min(enc.len());

            let full_dec = deobfuscate(&enc);
            let expected = &full_dec[start..end];

            let range_dec = deobfuscate_range(&enc[block_start..block_end], block_start);
            let actual = &range_dec[start - block_start..end - block_start];

            assert_eq!(actual, expected, "mismatch at start={start} end={end}");
            start += 777;
        }
    }
}
