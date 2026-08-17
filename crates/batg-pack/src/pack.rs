use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::PackError;
use crate::format::{self, ENTRY_SIZE, Entry, HEADER_SIZE};

struct Blob {
    entry: Entry,
    compressed: Vec<u8>
}

pub fn pack(output: &Path, mut adds: Vec<(String, PathBuf)>) -> Result<(), PackError> {
    if adds.is_empty() {
        return Err(PackError::Empty);
    }

    let duplicate = {
        let mut seen = HashSet::new();
        let mut duplicate = None;
        for (index, (triple, _)) in adds.iter().enumerate() {
            format::encode_triple(triple)?;
            if !seen.insert(triple.as_str()) {
                duplicate = Some(index);
                break;
            }
        }
        duplicate
    };
    if let Some(index) = duplicate {
        let (triple, _) = adds.swap_remove(index);
        return Err(PackError::DuplicateTriple(triple.into_boxed_str()));
    }

    let mut blobs = Vec::with_capacity(adds.len());
    for (target_triple, path) in adds {
        if !path.is_file() {
            return Err(PackError::NotFound(path));
        }

        let raw = fs::read(path)?;
        let checksum = crc32fast::hash(&raw);
        let compressed = zstd::encode_all(raw.as_slice(), 3)?;
        blobs.push(Blob {
            entry: Entry {
                target_triple,
                offset: 0,
                compressed_size: compressed.len() as u64,
                uncompressed_size: raw.len() as u64,
                checksum
            },
            compressed
        });
    }

    let mut offset = (HEADER_SIZE + blobs.len() * ENTRY_SIZE) as u64;
    for blob in &mut blobs {
        blob.entry.offset = offset;
        offset += blob.compressed.len() as u64;
    }

    let mut file = File::create(output)?;
    format::write_header(&mut file, blobs.iter().map(|blob| &blob.entry))?;
    for blob in &blobs {
        file.write_all(&blob.compressed)?;
    }

    Ok(())
}
