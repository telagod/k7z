use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use k7z_common::{
    ArchiveFormat, EntryMetadata, K7zError, ListReport, OverwriteMode, PackReport, PackRequest,
    Result, TestReport, TestRequest, UnpackReport, UnpackRequest, calculate_input_size, safe_join,
};
use tar::{Archive, Builder, EntryType};
use xz2::{read::XzDecoder, write::XzEncoder};
use zstd::stream::{Decoder as ZstdDecoder, Encoder as ZstdEncoder};

pub fn pack(req: &PackRequest) -> Result<PackReport> {
    if req.password.is_some() {
        return Err(K7zError::InvalidInput(
            "tar encryption is not supported in this version".to_string(),
        ));
    }
    if req.sources.is_empty() {
        return Err(K7zError::InvalidInput(
            "at least one source path is required".to_string(),
        ));
    }

    let entries = match req.format {
        ArchiveFormat::Tar => {
            let file = File::create(&req.output)?;
            let mut builder = Builder::new(file);
            let entries = append_sources(&mut builder, req)?;
            builder.finish()?;
            entries
        }
        ArchiveFormat::TarGz => {
            let file = File::create(&req.output)?;
            let encoder = GzEncoder::new(file, Compression::new(req.level.unwrap_or(6).min(9)));
            let mut builder = Builder::new(encoder);
            let entries = append_sources(&mut builder, req)?;
            let encoder = builder.into_inner()?;
            encoder.finish()?;
            entries
        }
        ArchiveFormat::TarXz => {
            let file = File::create(&req.output)?;
            let encoder = XzEncoder::new(file, req.level.unwrap_or(6).min(9));
            let mut builder = Builder::new(encoder);
            let entries = append_sources(&mut builder, req)?;
            let encoder = builder.into_inner()?;
            encoder.finish()?;
            entries
        }
        ArchiveFormat::TarZst => {
            let file = File::create(&req.output)?;
            let mut encoder = ZstdEncoder::new(file, req.level.unwrap_or(3) as i32)?;
            let mut builder = Builder::new(&mut encoder);
            let entries = append_sources(&mut builder, req)?;
            builder.finish()?;
            drop(builder);
            encoder.finish()?;
            entries
        }
        _ => {
            return Err(K7zError::UnsupportedFormat(format!(
                "tar crate cannot write format {}",
                req.format.as_str()
            )));
        }
    };

    Ok(PackReport {
        archive: req.output.clone(),
        entries,
        bytes_in: calculate_input_size(&req.sources)?,
        bytes_out: req.output.metadata()?.len(),
    })
}

pub fn unpack(req: &UnpackRequest, format: ArchiveFormat) -> Result<UnpackReport> {
    let file = File::open(&req.archive)?;
    match format {
        ArchiveFormat::Tar => unpack_from_reader(file, req),
        ArchiveFormat::TarGz => unpack_from_reader(GzDecoder::new(file), req),
        ArchiveFormat::TarXz => unpack_from_reader(XzDecoder::new(file), req),
        ArchiveFormat::TarZst => unpack_from_reader(ZstdDecoder::new(file)?, req),
        _ => Err(K7zError::UnsupportedFormat(format.as_str().to_string())),
    }
}

pub fn list(req: &k7z_common::ListRequest, format: ArchiveFormat) -> Result<ListReport> {
    let file = File::open(&req.archive)?;
    match format {
        ArchiveFormat::Tar => list_from_reader(file),
        ArchiveFormat::TarGz => list_from_reader(GzDecoder::new(file)),
        ArchiveFormat::TarXz => list_from_reader(XzDecoder::new(file)),
        ArchiveFormat::TarZst => list_from_reader(ZstdDecoder::new(file)?),
        _ => Err(K7zError::UnsupportedFormat(format.as_str().to_string())),
    }
}

pub fn test(req: &TestRequest, format: ArchiveFormat) -> Result<TestReport> {
    let file = File::open(&req.archive)?;
    match format {
        ArchiveFormat::Tar => test_from_reader(file),
        ArchiveFormat::TarGz => test_from_reader(GzDecoder::new(file)),
        ArchiveFormat::TarXz => test_from_reader(XzDecoder::new(file)),
        ArchiveFormat::TarZst => test_from_reader(ZstdDecoder::new(file)?),
        _ => Err(K7zError::UnsupportedFormat(format.as_str().to_string())),
    }
}

fn append_sources<W: Write>(builder: &mut Builder<W>, req: &PackRequest) -> Result<usize> {
    let mut entries = 0_usize;
    for source in &req.sources {
        if !source.exists() {
            return Err(K7zError::InvalidInput(format!(
                "source does not exist: {}",
                source.display()
            )));
        }
        let archive_name = source
            .file_name()
            .map(PathBuf::from)
            .ok_or_else(|| K7zError::InvalidInput(source.display().to_string()))?;

        if source.is_dir() {
            builder.append_dir_all(&archive_name, source)?;
            entries += count_tree(source)?;
        } else {
            builder.append_path_with_name(source, &archive_name)?;
            entries += 1;
        }
    }
    Ok(entries)
}

fn count_tree(root: &Path) -> Result<usize> {
    let mut count = 0_usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        count += 1;
        if path.is_dir() {
            for child in std::fs::read_dir(&path)? {
                stack.push(child?.path());
            }
        }
    }
    Ok(count)
}

fn unpack_from_reader<R: Read>(reader: R, req: &UnpackRequest) -> Result<UnpackReport> {
    std::fs::create_dir_all(&req.output_dir)?;
    let mut archive = Archive::new(reader);
    let mut entries = 0_usize;
    let mut bytes_out = 0_u64;

    for item in archive.entries()? {
        let mut entry = item?;
        let relative = entry.path()?.into_owned();
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(K7zError::InvalidInput(format!(
                "unsupported tar entry type for safety: {}",
                relative.display()
            )));
        }

        let destination = safe_join(&req.output_dir, &relative)?;
        if destination.exists() {
            match req.overwrite {
                OverwriteMode::Always => {}
                OverwriteMode::Never => continue,
                OverwriteMode::Ask => {
                    return Err(K7zError::AlreadyExists(destination.display().to_string()));
                }
            }
        }

        if entry_type == EntryType::Directory {
            std::fs::create_dir_all(&destination)?;
            entries += 1;
            continue;
        }
        if !entry_type.is_file() {
            continue;
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut output = File::create(&destination)?;
        let written = io::copy(&mut entry, &mut output)?;
        entries += 1;
        bytes_out = bytes_out.saturating_add(written);
    }

    Ok(UnpackReport {
        output_dir: req.output_dir.clone(),
        entries,
        bytes_out,
    })
}

pub fn list_from_reader<R: Read>(reader: R) -> Result<ListReport> {
    let mut archive = Archive::new(reader);
    let mut entries = Vec::new();
    for item in archive.entries()? {
        let entry = item?;
        let path = entry.path()?.to_string_lossy().to_string();
        let is_dir = entry.header().entry_type().is_dir();
        let size = if is_dir { 0 } else { entry.size() };
        entries.push(EntryMetadata {
            path,
            is_dir,
            size,
            compressed_size: None,
        });
    }
    Ok(ListReport { entries })
}

fn test_from_reader<R: Read>(reader: R) -> Result<TestReport> {
    let mut archive = Archive::new(reader);
    let mut sink = io::sink();
    let mut entries_checked = 0_usize;
    for item in archive.entries()? {
        let mut entry = item?;
        if entry.header().entry_type().is_file() {
            io::copy(&mut entry, &mut sink)?;
        }
        entries_checked += 1;
    }
    Ok(TestReport { entries_checked })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_from_reader_rejects_invalid_tar() {
        let cursor = io::Cursor::new(b"not-a-tar".to_vec());
        assert!(list_from_reader(cursor).is_err());
    }
}
