use std::fs::File;
use std::io;
use std::path::PathBuf;

use k7z_common::{
    EntryMetadata, K7zError, ListReport, OverwriteMode, PackReport, PackRequest, Result,
    TestReport, TestRequest, UnpackReport, UnpackRequest, calculate_input_size, safe_join,
};
use sevenz_rust2::{
    Archive, ArchiveReader, ArchiveWriter, Password,
    encoder_options::{AesEncoderOptions, Lzma2Options},
};
use walkdir::WalkDir;

fn seven_error(err: sevenz_rust2::Error) -> K7zError {
    K7zError::Other(format!("7z error: {err}"))
}

pub fn pack(req: &PackRequest) -> Result<PackReport> {
    if req.sources.is_empty() {
        return Err(K7zError::InvalidInput(
            "at least one source path is required".to_string(),
        ));
    }
    for source in &req.sources {
        if !source.exists() {
            return Err(K7zError::InvalidInput(format!(
                "source does not exist: {}",
                source.display()
            )));
        }
    }

    let mut writer = ArchiveWriter::create(&req.output).map_err(seven_error)?;
    let level = req.level.unwrap_or(6).min(9);
    if let Some(raw_password) = req.password.as_deref() {
        let password = Password::from(raw_password);
        writer.set_content_methods(vec![
            AesEncoderOptions::new(password).into(),
            Lzma2Options::from_level(level).into(),
        ]);
        writer.set_encrypt_header(true);
    } else {
        writer.set_content_methods(vec![Lzma2Options::from_level(level).into()]);
        writer.set_encrypt_header(false);
    }

    for source in &req.sources {
        if req.solid {
            writer
                .push_source_path(source, |_| true)
                .map_err(seven_error)?;
        } else {
            writer
                .push_source_path_non_solid(source, |_| true)
                .map_err(seven_error)?;
        }
    }
    writer.finish()?;

    Ok(PackReport {
        archive: req.output.clone(),
        entries: count_inputs(&req.sources)?,
        bytes_in: calculate_input_size(&req.sources)?,
        bytes_out: req.output.metadata()?.len(),
    })
}

pub fn unpack(req: &UnpackRequest) -> Result<UnpackReport> {
    std::fs::create_dir_all(&req.output_dir)?;
    let mut entries = 0_usize;
    let mut bytes_out = 0_u64;
    let mut reader = ArchiveReader::new(
        File::open(&req.archive)?,
        password_from(req.password.as_deref()),
    )
    .map_err(seven_error)?;

    reader
        .for_each_entries(|entry, content| {
            let relative = PathBuf::from(entry.name());
            let destination = safe_join(&req.output_dir, &relative)
                .map_err(|err| io::Error::other(err.to_string()))?;

            if entry.is_directory() {
                std::fs::create_dir_all(&destination)?;
                entries += 1;
                return Ok(true);
            }
            if destination.exists() {
                match req.overwrite {
                    OverwriteMode::Always => {}
                    OverwriteMode::Never => return Ok(true),
                    OverwriteMode::Ask => {
                        return Err(io::Error::other(format!(
                            "destination exists: {}",
                            destination.display()
                        ))
                        .into());
                    }
                }
            }
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = File::create(&destination)?;
            let written = io::copy(content, &mut file)?;
            bytes_out = bytes_out.saturating_add(written);
            entries += 1;
            Ok(true)
        })
        .map_err(seven_error)?;

    Ok(UnpackReport {
        output_dir: req.output_dir.clone(),
        entries,
        bytes_out,
    })
}

pub fn list(req: &k7z_common::ListRequest) -> Result<ListReport> {
    let archive =
        Archive::open_with_password(&req.archive, &password_from(req.password.as_deref()))
            .map_err(seven_error)?;
    let entries = archive
        .files
        .iter()
        .map(|entry| EntryMetadata {
            path: entry.name().to_string(),
            is_dir: entry.is_directory(),
            size: entry.size(),
            compressed_size: Some(entry.compressed_size),
        })
        .collect();
    Ok(ListReport { entries })
}

pub fn test(req: &TestRequest) -> Result<TestReport> {
    let mut reader = ArchiveReader::new(
        File::open(&req.archive)?,
        password_from(req.password.as_deref()),
    )
    .map_err(seven_error)?;
    let mut sink = io::sink();
    let mut entries_checked = 0_usize;
    reader
        .for_each_entries(|entry, content| {
            if !entry.is_directory() {
                io::copy(content, &mut sink)?;
            }
            entries_checked += 1;
            Ok(true)
        })
        .map_err(seven_error)?;
    Ok(TestReport { entries_checked })
}

fn password_from(raw: Option<&str>) -> Password {
    match raw {
        Some(value) if !value.is_empty() => Password::from(value),
        _ => Password::empty(),
    }
}

fn count_inputs(sources: &[PathBuf]) -> Result<usize> {
    let mut count = 0_usize;
    for source in sources {
        if source.is_file() {
            count += 1;
            continue;
        }
        for entry in WalkDir::new(source).follow_links(false) {
            let entry = entry.map_err(|err| K7zError::Other(err.to_string()))?;
            if entry.file_type().is_file() || entry.file_type().is_dir() {
                count += 1;
            }
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_password_is_default() {
        assert!(password_from(None).is_empty());
    }

    #[test]
    fn non_empty_password_is_used() {
        assert!(!password_from(Some("abc")).is_empty());
    }
}
