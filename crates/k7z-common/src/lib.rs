use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, K7zError>;

#[derive(Debug, Error)]
pub enum K7zError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("unsafe archive path: {0}")]
    PathTraversal(String),
    #[error("entry already exists: {0}")]
    AlreadyExists(String),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArchiveFormat {
    #[serde(rename = "7z", alias = "SevenZ")]
    SevenZ,
    #[serde(rename = "zip", alias = "Zip")]
    Zip,
    #[serde(rename = "tar", alias = "Tar")]
    Tar,
    #[serde(rename = "tar.gz", alias = "TarGz", alias = "tgz")]
    TarGz,
    #[serde(rename = "tar.xz", alias = "TarXz", alias = "txz")]
    TarXz,
    #[serde(rename = "tar.zst", alias = "TarZst", alias = "tzst")]
    TarZst,
    #[serde(rename = "zst", alias = "Zst")]
    Zst,
}

impl ArchiveFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SevenZ => "7z",
            Self::Zip => "zip",
            Self::Tar => "tar",
            Self::TarGz => "tar.gz",
            Self::TarXz => "tar.xz",
            Self::TarZst => "tar.zst",
            Self::Zst => "zst",
        }
    }
}

impl std::str::FromStr for ArchiveFormat {
    type Err = K7zError;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "7z" => Ok(Self::SevenZ),
            "zip" => Ok(Self::Zip),
            "tar" => Ok(Self::Tar),
            "tar.gz" | "tgz" => Ok(Self::TarGz),
            "tar.xz" | "txz" => Ok(Self::TarXz),
            "tar.zst" | "tzst" => Ok(Self::TarZst),
            "zst" => Ok(Self::Zst),
            _ => Err(K7zError::UnsupportedFormat(raw.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum OverwriteMode {
    Ask,
    Always,
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionSpec {
    pub enabled: bool,
    pub header_encrypted: bool,
    pub cipher: Option<String>,
    pub kdf: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetadata {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub compressed_size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PackRequest {
    pub sources: Vec<PathBuf>,
    pub output: PathBuf,
    pub format: ArchiveFormat,
    pub level: Option<u32>,
    pub solid: bool,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UnpackRequest {
    pub archive: PathBuf,
    pub output_dir: PathBuf,
    pub overwrite: OverwriteMode,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListRequest {
    pub archive: PathBuf,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TestRequest {
    pub archive: PathBuf,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BenchRequest {
    pub source: PathBuf,
    pub format: ArchiveFormat,
    pub level: Option<u32>,
    pub iterations: u32,
    pub warmup_iterations: u32,
    pub solid: bool,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TaskRequest {
    Pack(PackRequest),
    Unpack(UnpackRequest),
    List(ListRequest),
    Test(TestRequest),
    Bench(BenchRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackReport {
    pub archive: PathBuf,
    pub entries: usize,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnpackReport {
    pub output_dir: PathBuf,
    pub entries: usize,
    pub bytes_out: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListReport {
    pub entries: Vec<EntryMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReport {
    pub entries_checked: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchReport {
    pub format: ArchiveFormat,
    pub iterations: u32,
    pub warmup_iterations: u32,
    pub total_input_bytes: u64,
    pub total_output_bytes: u64,
    pub elapsed_ms: u128,
    pub throughput_mib_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Report {
    Pack(PackReport),
    Unpack(UnpackReport),
    List(ListReport),
    Test(TestReport),
    Bench(BenchReport),
}

pub fn detect_format_from_path(path: &Path) -> Option<ArchiveFormat> {
    let file_name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        return Some(ArchiveFormat::TarGz);
    }
    if file_name.ends_with(".tar.xz") || file_name.ends_with(".txz") {
        return Some(ArchiveFormat::TarXz);
    }
    if file_name.ends_with(".tar.zst") || file_name.ends_with(".tzst") {
        return Some(ArchiveFormat::TarZst);
    }

    match path.extension().and_then(OsStr::to_str) {
        Some(ext) if ext.eq_ignore_ascii_case("7z") => Some(ArchiveFormat::SevenZ),
        Some(ext) if ext.eq_ignore_ascii_case("zip") => Some(ArchiveFormat::Zip),
        Some(ext) if ext.eq_ignore_ascii_case("tar") => Some(ArchiveFormat::Tar),
        Some(ext) if ext.eq_ignore_ascii_case("zst") => Some(ArchiveFormat::Zst),
        _ => None,
    }
}

pub fn safe_join(base: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute() {
        return Err(K7zError::PathTraversal(relative.display().to_string()));
    }

    let mut output = base.to_path_buf();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => output.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(K7zError::PathTraversal(relative.display().to_string()));
            }
        }
    }
    Ok(output)
}

pub fn calculate_input_size(paths: &[PathBuf]) -> Result<u64> {
    let mut total = 0_u64;
    for path in paths {
        if path.is_file() {
            total = total.saturating_add(path.metadata()?.len());
        } else if path.is_dir() {
            for entry in walk(path)? {
                if entry.is_file() {
                    total = total.saturating_add(entry.metadata()?.len());
                }
            }
        }
    }
    Ok(total)
}

fn walk(root: &Path) -> Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut entries = Vec::new();
    while let Some(path) = pending.pop() {
        entries.push(path.clone());
        if path.is_dir() {
            for child in std::fs::read_dir(&path)? {
                let child = child?.path();
                pending.push(child);
            }
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_format_serializes_to_cli_style_strings() {
        assert_eq!(serde_json::to_string(&ArchiveFormat::SevenZ).expect("serialize"), "\"7z\"");
        assert_eq!(serde_json::to_string(&ArchiveFormat::Zip).expect("serialize"), "\"zip\"");
        assert_eq!(serde_json::to_string(&ArchiveFormat::Tar).expect("serialize"), "\"tar\"");
        assert_eq!(
            serde_json::to_string(&ArchiveFormat::TarGz).expect("serialize"),
            "\"tar.gz\""
        );
        assert_eq!(
            serde_json::to_string(&ArchiveFormat::TarXz).expect("serialize"),
            "\"tar.xz\""
        );
        assert_eq!(
            serde_json::to_string(&ArchiveFormat::TarZst).expect("serialize"),
            "\"tar.zst\""
        );
        assert_eq!(serde_json::to_string(&ArchiveFormat::Zst).expect("serialize"), "\"zst\"");
    }

    #[test]
    fn archive_format_deserializes_from_legacy_and_cli_strings() {
        assert_eq!(
            serde_json::from_str::<ArchiveFormat>("\"Zip\"").expect("deserialize"),
            ArchiveFormat::Zip
        );
        assert_eq!(
            serde_json::from_str::<ArchiveFormat>("\"zip\"").expect("deserialize"),
            ArchiveFormat::Zip
        );
        assert_eq!(
            serde_json::from_str::<ArchiveFormat>("\"TarGz\"").expect("deserialize"),
            ArchiveFormat::TarGz
        );
        assert_eq!(
            serde_json::from_str::<ArchiveFormat>("\"tar.gz\"").expect("deserialize"),
            ArchiveFormat::TarGz
        );
    }
}
