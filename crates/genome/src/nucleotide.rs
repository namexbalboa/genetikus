//! 2-bit nucleotide encoding.
//!
//! ```text
//! A = 00   C = 01   G = 10   T = 11
//! ```
//!
//! With 32 nucleotides per `u64`, the MC1R coding sequence (954 bp) fits in
//! 30 words = 240 bytes per genome. One million genomes ≈ 230 MiB — small
//! enough to live in DDR with comfort, large enough to demand cache-aware
//! access patterns.

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Nucleotide {
    A = 0b00,
    C = 0b01,
    G = 0b10,
    T = 0b11,
}

impl Nucleotide {
    /// Parse from an ASCII byte. `N` and lower-case are not supported here:
    /// the caller is expected to have masked / repaired such bases upstream
    /// (Lava-style: malformed input is the parser's job, not the engine's).
    #[inline]
    pub fn from_ascii(b: u8) -> Option<Self> {
        Some(match b {
            b'A' | b'a' => Self::A,
            b'C' | b'c' => Self::C,
            b'G' | b'g' => Self::G,
            b'T' | b't' => Self::T,
            _ => return None,
        })
    }

    #[inline]
    pub fn to_ascii(self) -> u8 {
        match self {
            Self::A => b'A',
            Self::C => b'C',
            Self::G => b'G',
            Self::T => b'T',
        }
    }

    #[inline]
    pub fn bits(self) -> u8 {
        self as u8
    }

    #[inline]
    pub fn from_bits(b: u8) -> Self {
        match b & 0b11 {
            0b00 => Self::A,
            0b01 => Self::C,
            0b10 => Self::G,
            _ => Self::T,
        }
    }

    /// Watson-Crick complement.
    #[inline]
    pub fn complement(self) -> Self {
        // A↔T, C↔G  ⇔  flip both bits.
        Self::from_bits(!self.bits() & 0b11)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_round_trip() {
        for n in [Nucleotide::A, Nucleotide::C, Nucleotide::G, Nucleotide::T] {
            assert_eq!(Nucleotide::from_ascii(n.to_ascii()), Some(n));
        }
    }

    #[test]
    fn bits_round_trip() {
        for b in 0..4u8 {
            assert_eq!(Nucleotide::from_bits(b).bits(), b);
        }
    }

    #[test]
    fn complement() {
        assert_eq!(Nucleotide::A.complement(), Nucleotide::T);
        assert_eq!(Nucleotide::C.complement(), Nucleotide::G);
        assert_eq!(Nucleotide::G.complement(), Nucleotide::C);
        assert_eq!(Nucleotide::T.complement(), Nucleotide::A);
    }
}
