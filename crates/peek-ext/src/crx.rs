//! CRX3 package validation and extraction.
//!
//! CRX3 binary header structure:
//! - 4 bytes: Magic "Cr24" (0x43, 0x72, 0x32, 0x34)
//! - 4 bytes: Version (u32 little-endian, must be 3)
//! - 4 bytes: Header length (u32 little-endian)
//! - Header length bytes: Header protobuf
//! - Remainder: Standard ZIP archive.

use std::fs::{self, File};
use std::io::{self, Cursor, Read, Seek};
use std::path::Path;
use zip::ZipArchive;

use crate::ExtError;

const CRX3_MAGIC: &[u8; 4] = b"Cr24";
const ZIP_MAGIC: &[u8; 4] = b"PK\x03\x04";

/// Extract a CRX3 or standard ZIP extension package into `destination_dir`.
pub fn extract_crx_or_zip(data: &[u8], destination_dir: &Path) -> Result<(), ExtError> {
    if data.len() < 4 {
        return Err(ExtError::InvalidPackage("payload too short".into()));
    }

    let zip_bytes: &[u8] = if &data[0..4] == CRX3_MAGIC {
        if data.len() < 12 {
            return Err(ExtError::InvalidPackage("crx3 header incomplete".into()));
        }
        let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
        if version != 3 {
            return Err(ExtError::InvalidPackage(format!(
                "unsupported crx version {version}, expected 3"
            )));
        }
        let header_len = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        let zip_offset = 12 + header_len;
        if data.len() < zip_offset {
            return Err(ExtError::InvalidPackage("crx3 payload truncated".into()));
        }
        &data[zip_offset..]
    } else if &data[0..4] == ZIP_MAGIC {
        data
    } else {
        return Err(ExtError::InvalidPackage(
            "unknown format, expected CRX3 or ZIP archive".into(),
        ));
    };

    let cursor = Cursor::new(zip_bytes);
    extract_zip_archive(cursor, destination_dir)
}

fn extract_zip_archive<R: Read + Seek>(reader: R, destination_dir: &Path) -> Result<(), ExtError> {
    fs::create_dir_all(destination_dir).map_err(|e| ExtError::Io(e))?;
    let mut archive = ZipArchive::new(reader).map_err(|e| ExtError::Zip(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| ExtError::Zip(e.to_string()))?;
        let Some(enclosed_name) = file.enclosed_name().map(|p| p.to_path_buf()) else {
            // Path traversal attempt (zip-slip) - skip unsafe files
            continue;
        };

        let outpath = destination_dir.join(enclosed_name);

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| ExtError::Io(e))?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|e| ExtError::Io(e))?;
                }
            }
            let mut outfile = File::create(&outpath).map_err(|e| ExtError::Io(e))?;
            io::copy(&mut file, &mut outfile).map_err(|e| ExtError::Io(e))?;
        }
    }

    Ok(())
}

/// Recursively copy an unpacked extension directory into the destination.
pub fn copy_unpacked_dir(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_child = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_unpacked_dir(&entry.path(), &dest_child)?;
        } else {
            fs::copy(entry.path(), dest_child)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    fn make_test_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut buf));
            let options = SimpleFileOptions::default();
            writer.start_file("manifest.json", options).unwrap();
            writer
                .write_all(br#"{"manifest_version":3,"name":"ZipExt","version":"1.0"}"#)
                .unwrap();
            writer.finish().unwrap();
        }
        buf
    }

    #[test]
    fn extracts_standard_zip() {
        let zip_bytes = make_test_zip();
        let tmp = tempdir().unwrap();
        extract_crx_or_zip(&zip_bytes, tmp.path()).unwrap();

        let manifest = tmp.path().join("manifest.json");
        assert!(manifest.exists());
    }

    #[test]
    fn extracts_crx3_package() {
        let zip_bytes = make_test_zip();
        let mut crx = Vec::new();
        crx.extend_from_slice(CRX3_MAGIC);
        crx.extend_from_slice(&3u32.to_le_bytes()); // version 3
        let header = b"fake-protobuf-header";
        crx.extend_from_slice(&(header.len() as u32).to_le_bytes());
        crx.extend_from_slice(header);
        crx.extend_from_slice(&zip_bytes);

        let tmp = tempdir().unwrap();
        extract_crx_or_zip(&crx, tmp.path()).unwrap();

        let manifest = tmp.path().join("manifest.json");
        assert!(manifest.exists());
    }
}
