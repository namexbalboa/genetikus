# genetikus

A high-throughput genomic simulation engine in Rust, with the orchestration
core lifted from Intel's [Lava](https://github.com/lava-nc/lava) neuromorphic
framework.

> **Goal:** scan **1 000 000 MC1R genomes against the canonical
> red-hair-color SNP panel in under 100 ms** on commodity x86-64.

## What's in here

```
crates/
├── magma/      Lava-inspired core — Process / Port / Channel / Lifecycle
├── runtime/    Fixed-step clock with phase barriers across worker threads
├── genome/     2-bit packed DNA buffer + bitwise variant matcher
├── geneio/     mmap FASTA / VCF readers
└── mc1r/       Test payload (MC1R locus + RHC SNPs) with a runnable example
```

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the full design and the
mapping back to Lava's `magma.core.process`, `magma.core.process.ports`,
and `magma.runtime`.

## Running

```bash
# Build the workspace.
cargo build --release

# Run the end-to-end MC1R diagnostic on 1 M synthesised individuals.
cargo run --release --example mc1r_diagnostic -- 1000000

# Microbenchmarks for the matcher kernel.
cargo bench -p genetikus-mc1r

# Full test suite.
cargo test --workspace
```

## Stack

| Concern              | Crate                            |
| -------------------- | -------------------------------- |
| Concurrency          | `crossbeam-channel`, `rayon`     |
| mmap I/O             | `memmap2`                        |
| Errors               | `thiserror`                      |
| Benchmarks           | `criterion`                      |
| Sync primitive       | `std::sync::Barrier` (phase gate)|

## Status

`v0` — core abstractions, packed-DNA matcher, and the MC1R payload are all
in place and tested. Production-grade I/O (`noodles-vcf`/`noodles-fasta`),
hand-rolled SIMD, and a `Compiler` step that picks a `ProcessModel` per
target backend are explicit non-goals for this iteration; see the
"What to build next" section in `ARCHITECTURE.md`.
