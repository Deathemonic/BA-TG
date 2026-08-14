use std::fs::File;
use std::path::Path;

use baad_utils::info;

use crate::error::PackError;
use crate::format;

pub fn list(path: &Path) -> Result<(), PackError> {
    let mut file = File::open(path)?;
    let flat = format::read_header(&mut file)?;

    info!(version = flat.version, "Flat container");
    info!(entries = flat.entries.len(), "Flat entries");
    for entry in flat.entries {
        info!(
            target_triple = entry.target_triple,
            offset = entry.offset,
            compressed = entry.compressed_size,
            uncompressed = entry.uncompressed_size,
            checksum = entry.checksum,
            "Flat entry"
        );
    }

    Ok(())
}
