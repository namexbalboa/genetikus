//! genetikus-geneio
//!
//! Tiny mmap-based readers for the two file formats the engine cares about:
//!
//!   * FASTA — reference sequences (e.g. the MC1R locus).
//!   * VCF   – variant records that get compiled into [`VariantMask`]s.
//!
//! Both readers map the file into memory with `memmap2` and parse on demand.
//! No allocation per record beyond what `String::from_utf8` requires for
//! header fields.
//!
//! For production-grade VCF (BCF, BGZF, tabix indexing) swap in `noodles`.
//! This module exists so the engine has a zero-dep, deterministic baseline.

pub mod fasta;
pub mod vcf;

pub use fasta::{FastaError, FastaRecord, MmapFasta};
pub use vcf::{MmapVcf, VcfError, VcfRecord};
