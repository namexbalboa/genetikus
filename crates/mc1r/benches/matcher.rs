//! Criterion benchmark for the variant matcher on the MC1R / RHC panel.
//!
//! ```bash
//! cargo bench -p genetikus-mc1r
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use rayon::prelude::*;

use genetikus_genome::{MatcherEngine, PackedDna};
use genetikus_mc1r::{rhc_masks, LOCUS_LEN_BP, RHC_SNPS};

fn make_population(n: usize) -> Vec<PackedDna> {
    (0..n)
        .into_par_iter()
        .map(|i| {
            let mut g = PackedDna::zeroed(LOCUS_LEN_BP);
            for (s_idx, snp) in RHC_SNPS.iter().enumerate() {
                let carrier = (i + s_idx) % 7 == 0;
                let base = if carrier { snp.alt } else { snp.reference };
                g.set(snp.locus_offset(), base);
            }
            g
        })
        .collect()
}

fn bench_matcher(c: &mut Criterion) {
    let masks = rhc_masks();

    for &n in &[10_000usize, 100_000, 1_000_000] {
        let pop = make_population(n);
        let engine = MatcherEngine::new(&masks);

        let mut grp = c.benchmark_group(format!("rhc_panel_n{}", n));
        grp.throughput(Throughput::Elements(n as u64));

        grp.bench_function("carrier_counts", |b| {
            b.iter(|| {
                let counts = engine.carrier_counts(black_box(&pop));
                black_box(counts);
            })
        });

        grp.finish();
    }
}

criterion_group!(benches, bench_matcher);
criterion_main!(benches);
