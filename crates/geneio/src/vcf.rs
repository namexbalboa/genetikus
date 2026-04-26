//! Minimal mmap VCF reader.
//!
//! Only handles the eight mandatory columns:
//! `CHROM POS ID REF ALT QUAL FILTER INFO`.
//! Header (`##` and `#CHROM`) lines are skipped. INFO is returned verbatim.
//!
//! For multi-sample / genotype-level data and BGZF/BCF support, swap in
//! `noodles-vcf`. This implementation is here so the engine can ingest a
//! ClinVar-style site VCF with no external deps.

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VcfError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed vcf at line {line}: {reason}")]
    Malformed { line: usize, reason: &'static str },
}

/// One site-level VCF record.
#[derive(Debug, Clone)]
pub struct VcfRecord {
    pub chrom: String,
    pub pos: u64,
    pub id: String,
    pub reference: String,
    pub alt: String,
    pub qual: String,
    pub filter: String,
    pub info: String,
}

/// Memory-mapped VCF file.
pub struct MmapVcf {
    mmap: Mmap,
}

impl MmapVcf {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, VcfError> {
        let f = File::open(path)?;
        // Safety: read-only mapping; see `MmapFasta::open`.
        let mmap = unsafe { Mmap::map(&f)? };
        Ok(Self { mmap })
    }

    pub fn records(&self) -> Result<Vec<VcfRecord>, VcfError> {
        let bytes: &[u8] = &self.mmap;
        let mut out = Vec::new();
        for (lineno, line) in bytes.split(|&b| b == b'\n').enumerate() {
            if line.is_empty() || line.starts_with(b"#") {
                continue;
            }
            let s = std::str::from_utf8(line).map_err(|_| VcfError::Malformed {
                line: lineno + 1,
                reason: "not utf-8",
            })?;
            let mut cols = s.split('\t');
            let chrom = cols.next().ok_or(VcfError::Malformed {
                line: lineno + 1,
                reason: "missing CHROM",
            })?;
            let pos = cols
                .next()
                .ok_or(VcfError::Malformed {
                    line: lineno + 1,
                    reason: "missing POS",
                })?
                .parse::<u64>()
                .map_err(|_| VcfError::Malformed {
                    line: lineno + 1,
                    reason: "POS not u64",
                })?;
            let id = cols.next().unwrap_or(".");
            let reference = cols.next().unwrap_or("");
            let alt = cols.next().unwrap_or("");
            let qual = cols.next().unwrap_or(".");
            let filter = cols.next().unwrap_or(".");
            let info = cols.next().unwrap_or(".");

            out.push(VcfRecord {
                chrom: chrom.to_string(),
                pos,
                id: id.to_string(),
                reference: reference.to_string(),
                alt: alt.to_string(),
                qual: qual.to_string(),
                filter: filter.to_string(),
                info: info.to_string(),
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_clinvar_style() {
        let body = b"##fileformat=VCFv4.2\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n16\t89919709\trs1805007\tC\tT\t.\tPASS\tGENE=MC1R;AA=R151C\n";
        let mut p = std::env::temp_dir();
        p.push("test_mc1r.vcf");
        std::fs::File::create(&p).unwrap().write_all(body).unwrap();

        let vcf = MmapVcf::open(&p).unwrap();
        let recs = vcf.records().unwrap();
        assert_eq!(recs.len(), 1);
        let r = &recs[0];
        assert_eq!(r.chrom, "16");
        assert_eq!(r.pos, 89919709);
        assert_eq!(r.id, "rs1805007");
        assert_eq!(r.reference, "C");
        assert_eq!(r.alt, "T");
        assert!(r.info.contains("R151C"));
    }
}
