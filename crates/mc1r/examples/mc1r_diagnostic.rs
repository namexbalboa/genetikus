//! End-to-end MC1R diagnostic.
//!
//! Synthesises a population of N individuals over the MC1R locus, injects
//! the canonical RHC variants at controllable allele frequencies, and runs
//! the parallel matcher to count carriers. Demonstrates:
//!
//!   * 2-bit packed DNA construction at scale.
//!   * Pre-computed `VariantMask`s.
//!   * Rayon-parallel scan over `[PackedDna]`.
//!   * Wall-clock time vs. the 100 ms / 1 M genomes target.
//!
//! Run with:
//!
//! ```bash
//! cargo run --release --example mc1r_diagnostic -- 1000000
//! ```

use std::time::Instant;

use rayon::prelude::*;

use genetikus_genome::{Nucleotide, PackedDna};
use genetikus_mc1r::{rhc_masks, LOCUS_LEN_BP, RHC_SNPS};

fn main() {
    let n_individuals: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000);

    println!("== genetikus MC1R diagnostic ==");
    println!("locus length     : {} bp", LOCUS_LEN_BP);
    println!("RHC panel        : {} SNPs", RHC_SNPS.len());
    println!("population       : {} individuals", n_individuals);

    // 1. Generate population. Each individual is a fresh PackedDna at the
    //    reference (all-A baseline; in production the reference would be
    //    loaded from FASTA via geneio::MmapFasta).
    //
    //    To make the run interesting we inject the alt allele at every SNP
    //    site for every (individual_idx % stride == snp_idx) — gives a
    //    deterministic, easily-checked carrier distribution.
    let t0 = Instant::now();
    let population: Vec<PackedDna> = (0..n_individuals)
        .into_par_iter()
        .map(|i| {
            let mut g = PackedDna::zeroed(LOCUS_LEN_BP);
            for (s_idx, snp) in RHC_SNPS.iter().enumerate() {
                let carrier = (i + s_idx) % 7 == 0; // ~14% carrier rate
                let base = if carrier { snp.alt } else { snp.reference };
                g.set(snp.locus_offset(), base);
                // Fill an unrelated base to demonstrate independence:
                if i % 1024 == 0 {
                    g.set(0, Nucleotide::G);
                }
            }
            g
        })
        .collect();
    let build_ms = t0.elapsed().as_millis();
    println!("\n[build]   {} ms ({} MiB)", build_ms, mem_estimate_mib(&population));

    // 2. Pre-compute masks once.
    let masks = rhc_masks();

    // 3. Parallel scan.
    let engine = genetikus_genome::MatcherEngine::new(&masks);
    let t1 = Instant::now();
    let counts = engine.carrier_counts(&population);
    let scan_ms = t1.elapsed().as_millis();
    println!("[scan]    {} ms", scan_ms);

    // 4. Report.
    println!("\nCarrier counts:");
    for (snp, c) in RHC_SNPS.iter().zip(counts.iter()) {
        let pct = (*c as f64) * 100.0 / n_individuals as f64;
        println!(
            "  {:11} ({:5})  carriers = {:>9}  ({:>5.2} %)",
            snp.rsid, snp.aa_change, c, pct
        );
    }

    // 5. Goal check.
    let target_ms = 100u128;
    let status = if scan_ms <= target_ms { "MET" } else { "MISSED" };
    println!(
        "\nTarget: {} M genomes scanned in <= {} ms — {}",
        n_individuals as f64 / 1e6,
        target_ms,
        status
    );
}

fn mem_estimate_mib(pop: &[PackedDna]) -> usize {
    if pop.is_empty() {
        return 0;
    }
    let bytes_per = pop[0].words().len() * std::mem::size_of::<u64>();
    bytes_per * pop.len() / (1024 * 1024)
}
