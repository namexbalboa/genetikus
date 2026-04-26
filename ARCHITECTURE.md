# genetikus — architecture

A Rust port of the architectural ideas from Intel's [Lava](https://github.com/lava-nc/lava)
neuromorphic framework, repurposed as a genomic simulation / diagnostic
engine. We "steal" three things from `lava.magma`:

| Lava                                | genetikus                          |
| ----------------------------------- | ---------------------------------- |
| `magma.core.process` (lifecycle)    | `magma::process`, `magma::lifecycle` |
| `magma.core.process.ports`          | `magma::port`                      |
| `magma.runtime` (fixed-step clock)  | `runtime::Runtime` + `RuntimeService` |

…and pair them with a Data-Oriented Design (DOD) genome layer (2-bit packed
DNA, bitwise variant masks) sized to hit **1 M MC1R genomes / < 100 ms** on
commodity x86-64.

## Workspace layout

```
crates/
├── magma/      Lava-inspired core: Process / Port / Channel / Lifecycle
├── runtime/    Fixed-step orchestrator (Runtime + RuntimeService phase loop)
├── genome/     2-bit packed DNA buffer + bitwise VariantMask matcher
├── geneio/     mmap FASTA / VCF readers
└── mc1r/       Test payload: locus + RHC SNPs, end-to-end example, bench
```

## Crate-by-crate mapping

### `magma` — core abstractions

| Concept           | File                  | Lava analogue                              |
| ----------------- | --------------------- | ------------------------------------------ |
| `Process` trait   | `process.rs`          | `AbstractProcess`                          |
| `ProcessModel`    | `process.rs`          | `AbstractProcessModel` (CPU/SIMD/Loihi)    |
| `Var<T>`          | `process.rs`          | `magma.core.process.variable.Var`          |
| `MgmtCommand`     | `lifecycle.rs`        | `MGMT_COMMAND` enum                        |
| `MgmtResponse`    | `lifecycle.rs`        | `MGMT_RESPONSE` enum                       |
| `ProcessState`    | `lifecycle.rs`        | implicit (we make it explicit + validated) |
| `InPort/OutPort`  | `port.rs`             | `AbstractIOPort`                           |
| `RefPort/VarPort` | `port.rs`             | `AbstractRVPort`                           |
| `Channel<T>`      | `channel.rs`          | `PyPyChannel` (semaphore ring buffer)      |

`Channel<T>` is a thin wrapper around `crossbeam-channel::bounded`, which
gives us the same blocking-on-full / blocking-on-empty semantics Lava
gets from POSIX semaphores — without `unsafe`.

### `runtime` — fixed-step orchestrator

| Concept            | File          | Lava analogue                  |
| ------------------ | ------------- | ------------------------------ |
| `Runtime`          | `runtime.rs`  | `lava.magma.runtime.Runtime`   |
| `RuntimeService`   | `service.rs`  | `runtime_services.RuntimeService` |
| `Phase`            | `phase.rs`    | `SPK / PRE_MGMT / LRN / POST_MGMT` |

Each `Process` runs on its own OS thread. Inside `RuntimeService` the
threads share an `Arc<std::sync::Barrier>`. After every phase of every
tick, every worker calls `barrier.wait()` — this is the explicit
equivalent of Lava's "wait for `Done` from all PMs before advancing".

```text
service ──cmd──▶ [worker_0] ─┐
                              │ Run{steps: 100}
service ──cmd──▶ [worker_1] ─┤
                              │
              ...             ▼
                       per tick × 4 phases:
                         proc.step(t)
                         barrier.wait()  ◀── implicit barrier in Lava,
                                             explicit here
```

For each tick the canonical phase order is:

```
Replication ─▶ Mutation ─▶ Selection ─▶ Reporting
   (SPK)       (PRE_MGMT)    (LRN)      (POST_MGMT)
```

Determinism: because every Process completes phase P at tick T before any
Process begins phase P+1, there are no read-modify-write races between
peer Processes within a tick. Cross-tick state is pushed through
`Channel`s, which carry their own happens-before edge.

### `genome` — DOD bit-packed DNA

`PackedDna` stores 32 nucleotides per `u64`:

```
  bit:   0 1   2 3   4 5   ...   62 63
        ┌─────┬─────┬─────┐    ┌───────┐
  word: │ A=00│ C=01│ G=10│... │  T=11 │
        └─────┴─────┴─────┘    └───────┘
        nuc 0  nuc 1  nuc 2     nuc 31
```

Memory budget for the MC1R/RHC payload:

```
  locus length         : ~3 099 bp
  words per genome     : 97 × u64  = 776 bytes
  population (1 M)     : ~740 MiB
  fits in DDR comfortably; per-thread working set fits in L2 on modern x86
```

`VariantMask::matches(genome)` reduces to **two `u64` ops**, no branches:

```rust
let w = genome.words()[mask.word_idx];
(w & mask.mask) == mask.expected_alt
```

`MatcherEngine` parallelises this across genomes via `rayon::par_iter`.
The hot loop is auto-vectorisable; LLVM widens it to AVX2/AVX-512 on
x86-64 without manual intrinsics. (Hand-rolled SIMD via the `wide` crate
is the next step if benchmarks demand it.)

### `geneio` — mmap I/O

`memmap2`-backed readers for FASTA and (a minimal site-level) VCF.
Both map the file read-only and parse on demand. For BCF / BGZF / tabix
indexing, swap in `noodles-fasta` and `noodles-vcf`.

### `mc1r` — payload

Locus and SNP constants (Ensembl `ENSG00000258839`, RefSeq `NM_002386.4`,
`16q24.3`, GRCh38 89,917,879–89,920,977) plus the canonical RHC panel:
`rs1805007`, `rs1805008`, `rs1805009`, `rs11547464`, `rs1110400`. The
`mc1r_diagnostic` example synthesises 1 M individuals at a fixed carrier
distribution and measures end-to-end build + scan time. The Criterion
bench in `benches/matcher.rs` isolates the scan kernel.

## Performance target

> Process **1 M MC1R genomes in < 100 ms** on commodity x86-64.

The matcher is dominated by sequential reads of `Vec<u64>` and a single
register-resident AND/eq. At 1 M × 5 SNPs that is ~5 M memory reads of
8 bytes each = 40 MiB of DRAM traffic, well inside the bandwidth budget
of a single modern CPU.

## What to build next (intentionally not in v0)

1. Hand-rolled SIMD via `wide` for the carrier-count reduction.
2. `noodles-vcf` integration behind a feature flag.
3. A `Compiler` step that, given a `RunCfg`, picks a `ProcessModel`
   (`cpu_scalar` / `cpu_simd` / future `gpu`) per Process — the missing
   half of the Lava interface/behavior split.
4. `RefPort`/`VarPort` request/response handling at the runtime level.
5. Aho-Corasick scanner for multi-base motifs (insertions / deletions
   beyond simple SNPs).
