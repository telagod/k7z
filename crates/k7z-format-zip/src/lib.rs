use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::Path;

use k7z_common::{
    EntryMetadata, K7zError, ListReport, OverwriteMode, PackReport, PackRequest, Result,
    TestReport, TestRequest, UnpackReport, UnpackRequest, calculate_input_size,
};
use walkdir::WalkDir;
use zip::{CompressionMethod, ZipArchive, ZipWriter, result::ZipError, write::FileOptions};

fn zip_error(err: ZipError) -> K7zError {
    K7zError::Other(format!("zip error: {err}"))
}

pub fn pack(req: &PackRequest) -> Result<PackReport> {
    if req.password.is_some() {
        return Err(K7zError::InvalidInput(
            "zip encryption is not implemented yet".to_string(),
        ));
    }
    if req.sources.is_empty() {
        return Err(K7zError::InvalidInput(
            "at least one source path is required".to_string(),
        ));
    }

    let output = File::create(&req.output)?;
    let mut writer = ZipWriter::new(output);
    let method = if req.level == Some(0) {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Deflated
    };
    let options = FileOptions::default().compression_method(method);

    let mut entries = 0_usize;
    for source in &req.sources {
        if !source.exists() {
            return Err(K7zError::InvalidInput(format!(
                "source does not exist: {}",
                source.display()
            )));
        }
        entries += add_source(&mut writer, source, options)?;
    }
    writer.finish().map_err(zip_error)?;

    Ok(PackReport {
        archive: req.output.clone(),
        entries,
        bytes_in: calculate_input_size(&req.sources)?,
        bytes_out: req.output.metadata()?.len(),
    })
}

pub fn unpack(req: &UnpackRequest) -> Result<UnpackReport> {
    std::fs::create_dir_all(&req.output_dir)?;
    let input = File::open(&req.archive)?;
    let mut archive = ZipArchive::new(input).map_err(zip_error)?;

    let mut entries = 0_usize;
    let mut bytes_out = 0_u64;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_error)?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| K7zError::PathTraversal(entry.name().to_string()))?
            .to_path_buf();
        let destination = req.output_dir.join(relative);

        if entry.is_dir() {
            std::fs::create_dir_all(&destination)?;
            entries += 1;
            continue;
        }

        if destination.exists() {
            match req.overwrite {
                OverwriteMode::Always => {}
                OverwriteMode::Never => continue,
                OverwriteMode::Ask => {
                    return Err(K7zError::AlreadyExists(destination.display().to_string()));
                }
            }
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&destination)?;
        let written = io::copy(&mut entry, &mut file)?;
        entries += 1;
        bytes_out = bytes_out.saturating_add(written);
    }

    Ok(UnpackReport {
        output_dir: req.output_dir.clone(),
        entries,
        bytes_out,
    })
}

pub fn list(req: &k7z_common::ListRequest) -> Result<ListReport> {
    let input = File::open(&req.archive)?;
    list_from_reader(input)
}

pub fn list_from_reader<R: Read + Seek>(reader: R) -> Result<ListReport> {
    let mut archive = ZipArchive::new(reader).map_err(zip_error)?;

    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(zip_error)?;
        entries.push(EntryMetadata {
            path: entry.name().to_string(),
            is_dir: entry.is_dir(),
            size: entry.size(),
            compressed_size: Some(entry.compressed_size()),
        });
    }
    Ok(ListReport { entries })
}

pub fn test(req: &TestRequest) -> Result<TestReport> {
    let input = File::open(&req.archive)?;
    let mut archive = ZipArchive::new(input).map_err(zip_error)?;

    let mut entries_checked = 0_usize;
    let mut sink = io::sink();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_error)?;
        if entry.is_file() {
            io::copy(&mut entry, &mut sink)?;
        }
        entries_checked += 1;
    }
    Ok(TestReport { entries_checked })
}

fn add_source(writer: &mut ZipWriter<File>, source: &Path, options: FileOptions) -> Result<usize> {
    let mut count = 0_usize;
    let source = source.canonicalize()?;
    let base = source.parent().unwrap_or(Path::new("/"));

    if source.is_file() {
        let name = normalize_name(base, &source)?;
        add_file(writer, &source, &name, options)?;
        return Ok(1);
    }

    for entry in WalkDir::new(&source).follow_links(false) {
        let entry = entry.map_err(|err| K7zError::Other(err.to_string()))?;
        let path = entry.path();
        let name = normalize_name(base, path)?;
        if entry.file_type().is_dir() {
            writer
                .add_directory(format!("{name}/"), options)
                .map_err(zip_error)?;
        } else if entry.file_type().is_file() {
            add_file(writer, path, &name, options)?;
        } else {
            continue;
        }
        count += 1;
    }

    Ok(count)
}

fn add_file(
    writer: &mut ZipWriter<File>,
    path: &Path,
    name: &str,
    options: FileOptions,
) -> Result<()> {
    writer.start_file(name, options).map_err(zip_error)?;
    let mut input = File::open(path)?;
    io::copy(&mut input, writer)?;
    Ok(())
}

fn normalize_name(base: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(base)
        .map_err(|_| K7zError::InvalidInput(path.display().to_string()))?;
    let name = relative.to_string_lossy().replace('\\', "/");
    if name.is_empty() {
        return Err(K7zError::InvalidInput(path.display().to_string()));
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn normalize_name_uses_forward_slashes() {
        let root = PathBuf::from("/tmp");
        let file = PathBuf::from("/tmp/abc/def.txt");
        let name = normalize_name(&root, &file).expect("name");
        assert_eq!(name, "abc/def.txt");
    }

    #[test]
    fn list_from_reader_rejects_invalid_zip() {
        let cursor = io::Cursor::new(b"not-a-zip".to_vec());
        assert!(list_from_reader(cursor).is_err());
    }
}
