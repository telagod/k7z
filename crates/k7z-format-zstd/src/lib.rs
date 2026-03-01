use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

use k7z_common::{
    EntryMetadata, K7zError, ListReport, OverwriteMode, PackReport, PackRequest, Result,
    TestReport, TestRequest, UnpackReport, UnpackRequest,
};

pub fn pack(req: &PackRequest) -> Result<PackReport> {
    if req.password.is_some() {
        return Err(K7zError::InvalidInput(
            "zstd stream encryption is not supported in this version".to_string(),
        ));
    }
    if req.sources.len() != 1 {
        return Err(K7zError::InvalidInput(
            ".zst format requires exactly one source file".to_string(),
        ));
    }
    let source = &req.sources[0];
    if !source.is_file() {
        return Err(K7zError::InvalidInput(
            ".zst format supports file input only".to_string(),
        ));
    }

    let mut input = File::open(source)?;
    let output = File::create(&req.output)?;
    let mut encoder = zstd::stream::Encoder::new(output, req.level.unwrap_or(3) as i32)?;
    let bytes_in = io::copy(&mut input, &mut encoder)?;
    encoder.finish()?;

    Ok(PackReport {
        archive: req.output.clone(),
        entries: 1,
        bytes_in,
        bytes_out: req.output.metadata()?.len(),
    })
}

pub fn unpack(req: &UnpackRequest) -> Result<UnpackReport> {
    std::fs::create_dir_all(&req.output_dir)?;
    let target_name = req
        .archive
        .file_stem()
        .ok_or_else(|| K7zError::InvalidInput(req.archive.display().to_string()))?;
    let output_file = req.output_dir.join(target_name);
    if output_file.exists() {
        match req.overwrite {
            OverwriteMode::Always => {}
            OverwriteMode::Never => {
                return Ok(UnpackReport {
                    output_dir: req.output_dir.clone(),
                    entries: 0,
                    bytes_out: 0,
                });
            }
            OverwriteMode::Ask => {
                return Err(K7zError::AlreadyExists(output_file.display().to_string()));
            }
        }
    }

    let input = File::open(&req.archive)?;
    let mut decoder = zstd::stream::Decoder::new(input)?;
    let mut output = File::create(&output_file)?;
    let bytes_out = io::copy(&mut decoder, &mut output)?;

    Ok(UnpackReport {
        output_dir: req.output_dir.clone(),
        entries: 1,
        bytes_out,
    })
}

pub fn list(req: &k7z_common::ListRequest) -> Result<ListReport> {
    let name = req
        .archive
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "stream".to_string());

    list_from_reader(
        File::open(&req.archive)?,
        &name,
        Some(req.archive.metadata()?.len()),
    )
}

pub fn list_from_reader<R: Read>(
    reader: R,
    name: &str,
    compressed_size: Option<u64>,
) -> Result<ListReport> {
    let mut decoder = zstd::stream::Decoder::new(reader)?;
    let mut sink = io::sink();
    let size = io::copy(&mut decoder, &mut sink)?;

    Ok(ListReport {
        entries: vec![EntryMetadata {
            path: name.to_string(),
            is_dir: false,
            size,
            compressed_size,
        }],
    })
}

pub fn test(req: &TestRequest) -> Result<TestReport> {
    test_from_reader(File::open(&req.archive)?)
}

pub fn test_from_reader<R: Read>(reader: R) -> Result<TestReport> {
    let mut decoder = zstd::stream::Decoder::new(reader)?;
    let mut sink = io::sink();
    io::copy(&mut decoder, &mut sink)?;
    Ok(TestReport { entries_checked: 1 })
}

pub fn default_unpack_target(archive: &std::path::Path) -> Result<PathBuf> {
    archive
        .file_stem()
        .map(PathBuf::from)
        .ok_or_else(|| K7zError::InvalidInput(archive.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_from_reader_rejects_invalid_zstd() {
        let cursor = io::Cursor::new(b"not-a-zstd-stream".to_vec());
        assert!(list_from_reader(cursor, "stream", None).is_err());
    }
}
