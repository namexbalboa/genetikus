//! genetikus-mc1r
//!
//! MC1R locus constants and the canonical "RHC" (red-hair-color) SNP set,
//! used as the default test payload for the engine.
//!
//! Sources (all annotations are 0-based offsets *into the locus reference
//! buffer* the engine carries — they are NOT the GRCh38 chromosome
//! coordinates, which are stored on the side as `grch38_pos`):
//!
//!   * Ensembl gene: ENSG00000258839
//!   * RefSeq mRNA:  NM_002386.4
//!   * Locus:        chr16:89,917,879-89,920,977 (GRCh38, +strand)
//!   * Length:       3 099 bp (UTR + CDS, single coding exon)
//!
//! GRCh38 coordinates and ClinVar VCV IDs should be treated as the
//! ground-truth source — the in-buffer offsets are derived from them at
//! load time. See `grch38_to_offset` for the mapping.
//!
//! See: <https://www.ncbi.nlm.nih.gov/clinvar/>, <https://www.ensembl.org/>.

use genetikus_genome::{Nucleotide, VariantMask};

/// Ensembl gene identifier.
pub const ENSEMBL_ID: &str = "ENSG00000258839";

/// RefSeq mRNA accession.
pub const REFSEQ_MRNA: &str = "NM_002386.4";

/// Cytogenetic locus.
pub const LOCUS: &str = "16q24.3";

/// GRCh38 start coordinate (1-based, inclusive).
pub const GRCH38_START: u64 = 89_917_879;

/// GRCh38 end coordinate (1-based, inclusive).
pub const GRCH38_END: u64 = 89_920_977;

/// Buffer length (bp) for one MC1R locus copy in the engine.
pub const LOCUS_LEN_BP: usize = (GRCH38_END - GRCH38_START + 1) as usize;

/// One canonical RHC SNP.
#[derive(Debug, Clone, Copy)]
pub struct RhcSnp {
    /// dbSNP rsID, e.g. `"rs1805007"`.
    pub rsid: &'static str,
    /// Protein-level annotation, e.g. `"R151C"`.
    pub aa_change: &'static str,
    /// 1-based GRCh38 position on chr16.
    pub grch38_pos: u64,
    /// Reference base.
    pub reference: Nucleotide,
    /// Alt base (the variant we look for).
    pub alt: Nucleotide,
}

impl RhcSnp {
    /// 0-based offset within the MC1R locus buffer.
    pub const fn locus_offset(&self) -> usize {
        (self.grch38_pos - GRCH38_START) as usize
    }

    /// Pre-computed bitwise mask for the matcher.
    pub fn to_mask(&self) -> VariantMask {
        VariantMask::new(self.rsid, self.locus_offset(), self.alt)
    }
}

/// The canonical "red hair color" SNP panel.
///
/// GRCh38 positions are approximate to within a few bp — verify against
/// dbSNP at ingestion time before promoting to a clinical pipeline.
pub const RHC_SNPS: &[RhcSnp] = &[
    RhcSnp {
        rsid: "rs1805007",
        aa_change: "R151C",
        grch38_pos: 89_919_709,
        reference: Nucleotide::C,
        alt: Nucleotide::T,
    },
    RhcSnp {
        rsid: "rs1805008",
        aa_change: "R160W",
        grch38_pos: 89_919_736,
        reference: Nucleotide::C,
        alt: Nucleotide::T,
    },
    RhcSnp {
        rsid: "rs1805009",
        aa_change: "D294H",
        grch38_pos: 89_920_138,
        reference: Nucleotide::G,
        alt: Nucleotide::C,
    },
    RhcSnp {
        rsid: "rs11547464",
        aa_change: "R142H",
        grch38_pos: 89_919_682,
        reference: Nucleotide::G,
        alt: Nucleotide::A,
    },
    RhcSnp {
        rsid: "rs1110400",
        aa_change: "I155T",
        grch38_pos: 89_919_719,
        reference: Nucleotide::T,
        alt: Nucleotide::C,
    },
];

/// Build the full mask vector for the RHC panel.
pub fn rhc_masks() -> Vec<VariantMask> {
    RHC_SNPS.iter().map(|s| s.to_mask()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use genetikus_genome::PackedDna;

    #[test]
    fn locus_offsets_in_range() {
        for s in RHC_SNPS {
            assert!(s.locus_offset() < LOCUS_LEN_BP, "{} OOB", s.rsid);
        }
    }

    #[test]
    fn mask_round_trip_on_alt_genome() {
        let snp = RHC_SNPS[0]; // rs1805007
        let mut g = PackedDna::zeroed(LOCUS_LEN_BP);
        g.set(snp.locus_offset(), snp.alt);
        assert!(snp.to_mask().matches(&g));
    }

    #[test]
    fn mask_misses_on_reference_genome() {
        let snp = RHC_SNPS[0];
        let mut g = PackedDna::zeroed(LOCUS_LEN_BP);
        g.set(snp.locus_offset(), snp.reference);
        assert!(!snp.to_mask().matches(&g));
    }
}
