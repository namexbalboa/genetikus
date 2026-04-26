//! Variant matcher — bitwise AND in registers, scaled by Rayon.
//!
//! For a SNP at position `p` with reference base `R` and alt base `A`, we
//! pre-compute:
//!
//! ```text
//! mask     = 0b11 << ((p % 32) * 2)
//! expected = (A as u64) << ((p % 32) * 2)
//! word_idx = p / 32
//! ```
//!
//! Carrier check at runtime is then:
//!
//! ```text
//! (genome.words[word_idx] & mask) == expected   // alt allele present
//! ```
//!
//! Two `u64` ops, no branches. With Rayon we map this across N genomes, and
//! within each genome across M masks. For the MC1R/RHC payload (~5 SNPs over
//! ~3 kb genome × 1M individuals) this is well under the 100 ms target on
//! commodity x86-64.

use rayon::prelude::*;

use crate::nucleotide::Nucleotide;
use crate::packed::{PackedDna, BASES_PER_WORD};

/// A precomputed bitwise predicate for one genomic site.
///
/// `expected_alt` is the *alternate* allele — `matches` returns `true`
/// when the genome's base at `pos` equals the alt (i.e. the variant
/// is present in this individual).
#[derive(Debug, Clone)]
pub struct VariantMask {
    /// Stable identifier (e.g. `"rs1805007"`).
    pub id: String,
    /// 0-based position within the genome buffer.
    pub pos: usize,
    /// Word index into `PackedDna::words()`.
    pub word_idx: usize,
    /// Bit-mask isolating the two bits at `pos`.
    pub mask: u64,
    /// Expected payload at those two bits if the alt is present.
    pub expected_alt: u64,
}

impl VariantMask {
    pub fn new(id: impl Into<String>, pos: usize, alt: Nucleotide) -> Self {
        let word_idx = pos / BASES_PER_WORD;
        let off = (pos % BASES_PER_WORD) * 2;
        Self {
            id: id.into(),
            pos,
            word_idx,
            mask: 0b11u64 << off,
            expected_alt: (alt.bits() as u64) << off,
        }
    }

    /// Test a single genome.
    #[inline]
    pub fn matches(&self, genome: &PackedDna) -> bool {
        // Bounds-checked once. Hot loops in `MatcherEngine` call this in
        // a tight inner loop where the bounds check vanishes after inlining.
        let w = genome.words().get(self.word_idx).copied().unwrap_or(0);
        (w & self.mask) == self.expected_alt
    }
}

/// One match between a genome and a mask.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MatchHit {
    pub genome_idx: usize,
    pub mask_idx: usize,
}

/// Embarrassingly-parallel matcher over `[PackedDna] × [VariantMask]`.
pub struct MatcherEngine<'a> {
    masks: &'a [VariantMask],
}

impl<'a> MatcherEngine<'a> {
    pub fn new(masks: &'a [VariantMask]) -> Self {
        Self { masks }
    }

    /// Scan one genome, return the index of every matching mask.
    pub fn scan_one(&self, genome: &PackedDna) -> Vec<usize> {
        self.masks
            .iter()
            .enumerate()
            .filter_map(|(i, m)| m.matches(genome).then_some(i))
            .collect()
    }

    /// Scan many genomes in parallel; returns a flat vector of hits.
    ///
    /// Output ordering: stable by `genome_idx`, then by `mask_idx`.
    pub fn scan_many(&self, genomes: &[PackedDna]) -> Vec<MatchHit> {
        genomes
            .par_iter()
            .enumerate()
            .flat_map_iter(|(gi, g)| {
                self.masks
                    .iter()
                    .enumerate()
                    .filter_map(move |(mi, m)| {
                        m.matches(g).then_some(MatchHit {
                            genome_idx: gi,
                            mask_idx: mi,
                        })
                    })
            })
            .collect()
    }

    /// Per-mask carrier counts across all genomes, in parallel.
    pub fn carrier_counts(&self, genomes: &[PackedDna]) -> Vec<u64> {
        let n = self.masks.len();
        genomes
            .par_iter()
            .map(|g| {
                let mut row = vec![0u64; n];
                for (i, m) in self.masks.iter().enumerate() {
                    if m.matches(g) {
                        row[i] = 1;
                    }
                }
                row
            })
            .reduce(
                || vec![0u64; n],
                |mut a, b| {
                    for i in 0..n {
                        a[i] += b[i];
                    }
                    a
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dna(s: &str) -> PackedDna {
        PackedDna::from_ascii(s.as_bytes()).unwrap()
    }

    #[test]
    fn detects_alt_at_position() {
        // Reference: AAAAA  (all A); variant: A->T at position 2.
        let g_ref = dna("AAAAA");
        let g_alt = dna("AATAA");
        let m = VariantMask::new("test", 2, Nucleotide::T);
        assert!(!m.matches(&g_ref));
        assert!(m.matches(&g_alt));
    }

    #[test]
    fn scan_many_parallel() {
        let masks = vec![
            VariantMask::new("v0", 0, Nucleotide::T),
            VariantMask::new("v1", 31, Nucleotide::G),
        ];
        let engine = MatcherEngine::new(&masks);

        let mut g_alt = dna("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"); // 32 A
        g_alt.set(0, Nucleotide::T);
        g_alt.set(31, Nucleotide::G);

        let g_ref = dna("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");

        let genomes = vec![g_alt, g_ref];
        let hits = engine.scan_many(&genomes);

        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|h| h.genome_idx == 0));
    }

    #[test]
    fn carrier_counts_sum() {
        let masks = vec![VariantMask::new("v0", 1, Nucleotide::C)];
        let engine = MatcherEngine::new(&masks);

        let g0 = dna("ACAA");
        let g1 = dna("AAAA");
        let g2 = dna("ACAA");
        let counts = engine.carrier_counts(&[g0, g1, g2]);

        assert_eq!(counts, vec![2]);
    }
}
