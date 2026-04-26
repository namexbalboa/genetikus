//! Minimal mmap FASTA reader.

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;
use thiserror::Error;

use genetikus_genome::PackedDna;

#[derive(Debug, Error)]
pub enum FastaError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed fasta: {0}")]
    Malformed(&'static str),
    #[error("packing failed: {0}")]
    Pack(#[from] genetikus_genome::PackError),
}

/// One FASTA record. The sequence is materialised into a [`PackedDna`].
pub struct FastaRecord {
    pub id: String,
    pub description: String,
    pub seq: PackedDna,
}

/// Memory-mapped FASTA file. Iteration yields [`FastaRecord`]s.
pub struct MmapFasta {
    mmap: Mmap,
}

impl MmapFasta {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, FastaError> {
        let f = File::open(path)?;
        // Safety: we never write to the mmap, and we don't share the file
        // descriptor across processes that might truncate it.
        let mmap = unsafe { Mmap::map(&f)? };
        Ok(Self { mmap })
    }

    /// Parse all records up-front. For very large files, prefer a streaming
    /// iterator (left as future work; the `noodles-fasta` crate is the
    /// industrial-grade option).
    pub fn records(&self) -> Result<Vec<FastaRecord>, FastaError> {
        let mut out = Vec::new();
        let bytes: &[u8] = &self.mmap;

        let mut cur = 0;
        while cur < bytes.len() {
            // Header line.
            if bytes[cur] != b'>' {
                return Err(FastaError::Malformed("expected '>'"));
            }
            let header_end = bytes[cur..]
                .iter()
                .position(|&b| b == b'\n')
                .ok_or(FastaError::Malformed("header has no newline"))?
                + cur;
            let header = &bytes[cur + 1..header_end];
            let (id, description) = split_header(header);

            // Sequence: every line until the next '>' or EOF.
            let mut seq_buf: Vec<u8> = Vec::new();
            let mut i = header_end + 1;
            while i < bytes.len() && bytes[i] != b'>' {
                let line_end = bytes[i..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|p| p + i)
                    .unwrap_or(bytes.len());
                seq_buf.extend(
                    bytes[i..line_end]
                        .iter()
                        .copied()
                        .filter(|b| !b.is_ascii_whitespace()),
                );
                i = line_end + 1;
            }

            out.push(FastaRecord {
                id,
                description,
                seq: PackedDna::from_ascii(&seq_buf)?,
            });
            cur = i;
        }
        Ok(out)
    }
}

fn split_header(h: &[u8]) -> (String, String) {
    let space = h.iter().position(|&b| b == b' ');
    match space {
        Some(s) => (
            String::from_utf8_lossy(&h[..s]).into_owned(),
            String::from_utf8_lossy(&h[s + 1..]).into_owned(),
        ),
        None => (String::from_utf8_lossy(h).into_owned(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(name: &str, body: &[u8]) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(body).unwrap();
        p
    }

    #[test]
    fn parses_two_records() {
        let body = b">chr16 MC1R\nACGT\nACGT\n>rs1805007 alt\nTTTT\n";
        let path = write_tmp("test_two.fa", body);
        let fasta = MmapFasta::open(&path).unwrap();
        let recs = fasta.records().unwrap();
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].id, "chr16");
        assert_eq!(recs[0].description, "MC1R");
        assert_eq!(recs[0].seq.len(), 8);
        assert_eq!(recs[1].id, "rs1805007");
        assert_eq!(recs[1].seq.len(), 4);
    }
}
