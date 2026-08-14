use std::io::{Read, Write};

use crate::error::PackError;

pub const MAGIC: &[u8; 4] = b"FLAT";
pub const VERSION: u16 = 1;
pub const TRIPLE_LEN: usize = 32;
pub const ENTRY_SIZE: usize = 60;
pub const HEADER_SIZE: usize = 8;

pub struct Entry {
    pub target_triple: String,
    pub offset: u64,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub checksum: u32
}

pub struct FlatFile {
    pub version: u16,
    pub entries: Vec<Entry>
}

pub fn encode_triple(triple: &str) -> Result<[u8; TRIPLE_LEN], PackError> {
    let bytes = triple.as_bytes();
    if bytes.is_empty() || bytes.len() >= TRIPLE_LEN || bytes.contains(&0) {
        return Err(PackError::InvalidTriple);
    }

    let mut out = [0u8; TRIPLE_LEN];
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(out)
}

pub fn decode_triple(bytes: &[u8; TRIPLE_LEN]) -> Result<String, PackError> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(TRIPLE_LEN);
    if bytes[end..].iter().any(|&b| b != 0) {
        return Err(PackError::Invalid);
    }
    let triple = std::str::from_utf8(&bytes[..end])?;
    if triple.is_empty() {
        return Err(PackError::InvalidTriple);
    }

    Ok(triple.to_owned())
}

pub fn write_header<'a, W, I>(writer: &mut W, entries: I) -> Result<(), PackError>
where
    W: Write,
    I: ExactSizeIterator<Item = &'a Entry>
{
    writer.write_all(MAGIC)?;
    writer.write_all(&VERSION.to_le_bytes())?;
    writer.write_all(&(u16::try_from(entries.len()).map_err(|_| PackError::Invalid)?).to_le_bytes())?;

    for entry in entries {
        writer.write_all(&encode_triple(&entry.target_triple)?)?;
        writer.write_all(&entry.offset.to_le_bytes())?;
        writer.write_all(&entry.compressed_size.to_le_bytes())?;
        writer.write_all(&entry.uncompressed_size.to_le_bytes())?;
        writer.write_all(&entry.checksum.to_le_bytes())?;
    }

    Ok(())
}

pub fn read_header<R: Read>(reader: &mut R) -> Result<FlatFile, PackError> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(PackError::Invalid);
    }

    let mut short = [0u8; 2];
    reader.read_exact(&mut short)?;
    let version = u16::from_le_bytes(short);
    if version != VERSION {
        return Err(PackError::UnsupportedVersion(version));
    }

    reader.read_exact(&mut short)?;
    let count = u16::from_le_bytes(short);
    if count == 0 {
        return Err(PackError::Empty);
    }

    let mut entries = Vec::with_capacity(count as usize);
    let mut buf = [0u8; ENTRY_SIZE];
    for _ in 0..count {
        reader.read_exact(&mut buf)?;
        entries.push(parse_entry(&buf)?);
    }

    Ok(FlatFile { version, entries })
}

fn parse_entry(buf: &[u8; ENTRY_SIZE]) -> Result<Entry, PackError> {
    let triple: [u8; TRIPLE_LEN] = buf[..TRIPLE_LEN].try_into().map_err(|_| PackError::Invalid)?;
    let offset = u64::from_le_bytes(buf[32..40].try_into().map_err(|_| PackError::Invalid)?);
    let compressed_size = u64::from_le_bytes(buf[40..48].try_into().map_err(|_| PackError::Invalid)?);
    let uncompressed_size = u64::from_le_bytes(buf[48..56].try_into().map_err(|_| PackError::Invalid)?);
    let checksum = u32::from_le_bytes(buf[56..60].try_into().map_err(|_| PackError::Invalid)?);

    Ok(Entry {
        target_triple: decode_triple(&triple)?,
        offset,
        compressed_size,
        uncompressed_size,
        checksum
    })
}
