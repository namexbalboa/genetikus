//! genetikus-genome
//!
//! Data-Oriented Design building blocks for genomic data.
//!
//!   * [`Nucleotide`] – 2-bit alphabet `A=00, C=01, G=10, T=11`.
//!   * [`PackedDna`]  – `Vec<u64>` storing 32 nucleotides per word.
//!   * [`VariantMask`] – a (mask, expected_word) pair; matching becomes
//!                        `(genome & mask) == expected`, a single bitwise op.
//!   * [`MatcherEngine`] – scans many genomes against many masks in parallel
//!                          via Rayon.

pub mod mask;
pub mod nucleotide;
pub mod packed;

pub use mask::{MatcherEngine, MatchHit, VariantMask};
pub use nucleotide::Nucleotide;
pub use packed::{PackedDna, PackError};
