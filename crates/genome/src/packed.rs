//! 2-bit packed DNA buffer.
//!
//! 32 nucleotides per `u64`, low-order bits first:
//!
//! ```text
//! word_idx = i / 32        offset = (i % 32) * 2
//! word     = words[word_idx]
//! base     = (word >> offset) & 0b11
//! ```
//!
//! All operations are designed to be auto-vectorisable on x86-64 / AArch64
//! by the LLVM back-end (no manual `unsafe` SIMD intrinsics required for
//! the base layer; the matcher in `mask.rs` performs bitwise AND in chunks
//! that the optimiser will widen to `pcmpeqq` / `vpand`).

use thiserror::Error;

use crate::nucleotide::Nucleotide;

#[derive(Debug, Error)]
pub enum PackError {
    #[error("non-ACGT byte at position {pos}: {byte:#04x}")]
    BadBase { pos: usize, byte: u8 },
}

/// Number of bases packed into one `u64` word.
pub const BASES_PER_WORD: usize = 32;

/// Bit-packed DNA sequence backed by a contiguous `Vec<u64>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedDna {
    /// Packed words. Bases past `len` within the last word are zero.
    words: Vec<u64>,
    /// Logical length in bases (not words).
    len: usize,
}

impl PackedDna {
    /// New buffer with `len` bases of A (all zero).
    pub fn zeroed(len: usize) -> Self {
        let n_words = len.div_ceil(BASES_PER_WORD);
        Self {
            words: vec![0u64; n_words],
            len,
        }
    }

    /// Pack from an ASCII slice. Returns the first non-ACGT position on error.
    pub fn from_ascii(bytes: &[u8]) -> Result<Self, PackError> {
        let mut buf = Self::zeroed(bytes.len());
        for (i, &b) in bytes.iter().enumerate() {
            let n = Nucleotide::from_ascii(b)
                .ok_or(PackError::BadBase { pos: i, byte: b })?;
            buf.set(i, n);
        }
        Ok(buf)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn words(&self) -> &[u64] {
        &self.words
    }

    #[inline]
    pub fn words_mut(&mut self) -> &mut [u64] {
        &mut self.words
    }

    #[inline]
    pub fn get(&self, idx: usize) -> Nucleotide {
        debug_assert!(idx < self.len);
        let word = self.words[idx / BASES_PER_WORD];
        let off = (idx % BASES_PER_WORD) * 2;
        Nucleotide::from_bits(((word >> off) & 0b11) as u8)
    }

    #[inline]
    pub fn set(&mut self, idx: usize, n: Nucleotide) {
        debug_assert!(idx < self.len);
        let w = idx / BASES_PER_WORD;
        let off = (idx % BASES_PER_WORD) * 2;
        let mask = 0b11u64 << off;
        self.words[w] = (self.words[w] & !mask) | ((n.bits() as u64) << off);
    }

    /// Decode back to an ASCII string. Mostly for tests.
    pub fn to_ascii(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.len);
        for i in 0..self.len {
            out.push(self.get(i).to_ascii());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_ascii() {
        let s = b"ACGTACGTACGTACGTACGTACGTACGTACGT"; // 32 bases, exactly 1 word
        let p = PackedDna::from_ascii(s).unwrap();
        assert_eq!(p.len(), 32);
        assert_eq!(p.words().len(), 1);
        assert_eq!(p.to_ascii(), s);
    }

    #[test]
    fn cross_word_boundary() {
        let s: Vec<u8> = (0..40).map(|i| b"ACGT"[i % 4]).collect();
        let p = PackedDna::from_ascii(&s).unwrap();
        assert_eq!(p.len(), 40);
        assert_eq!(p.words().len(), 2);
        for i in 0..40 {
            assert_eq!(p.get(i).to_ascii(), s[i], "mismatch at {i}");
        }
    }

    #[test]
    fn set_idempotent() {
        let mut p = PackedDna::zeroed(8);
        p.set(3, Nucleotide::T);
        p.set(3, Nucleotide::T);
        assert_eq!(p.get(3), Nucleotide::T);
        assert_eq!(p.get(2), Nucleotide::A);
        assert_eq!(p.get(4), Nucleotide::A);
    }

    #[test]
    fn rejects_non_acgt() {
        let err = PackedDna::from_ascii(b"ACGTN").unwrap_err();
        match err {
            PackError::BadBase { pos, byte } => {
                assert_eq!(pos, 4);
                assert_eq!(byte, b'N');
            }
        }
    }
}
