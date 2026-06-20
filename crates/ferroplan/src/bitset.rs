//! Fixed-width bit set over `u64` words — the fact layer of a state.
//! Word-oriented so applicability/apply are tight bitwise loops and states are
//! compact for hashing/dedup in parallel search.

#[inline]
pub fn words_for(n_bits: usize) -> usize {
    (n_bits + 63) / 64
}

#[inline]
pub fn test(w: &[u64], i: usize) -> bool {
    (w[i >> 6] >> (i & 63)) & 1 != 0
}

#[inline]
pub fn set(w: &mut [u64], i: usize) {
    w[i >> 6] |= 1u64 << (i & 63);
}

#[inline]
pub fn clear(w: &mut [u64], i: usize) {
    w[i >> 6] &= !(1u64 << (i & 63));
}

/// Count set bits (population count) across the word array.
pub fn count(w: &[u64]) -> usize {
    w.iter().map(|x| x.count_ones() as usize).sum()
}
