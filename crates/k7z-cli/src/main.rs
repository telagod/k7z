use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand, ValueEnum};
use k7z_common::{
    ArchiveFormat, BenchRequest, K7zError, ListRequest, OverwriteMode, PackRequest, Report,
    TaskRequest, TestRequest, UnpackRequest, detect_format_from_path,
};
use miette::{IntoDiagnostic, miette};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "k7z",
    version,
    about = "Rust archive tool for 7z/zstd/zip/tar.*"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(alias = "a")]
    Pack(PackArgs),
    #[command(alias = "x")]
    Unpack(UnpackArgs),
    #[command(alias = "l")]
    List(ListArgs),
    #[command(alias = "t")]
    Test(TestArgs),
    Bench(BenchArgs),
}

#[derive(Debug, clap::Args)]
struct PackArgs {
    #[arg(value_name = "SOURCE", required = true)]
    sources: Vec<PathBuf>,
    #[arg(short = 'o', long = "output", value_name = "ARCHIVE")]
    output: PathBuf,
    #[arg(long = "format", short = 'f')]
    format: Option<String>,
    #[arg(long = "level", alias = "mx")]
    level: Option<u32>,
    #[arg(long = "solid", default_value_t = false)]
    solid: bool,
    #[arg(short = 'p', long = "password")]
    password: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OverwriteArg {
    Ask,
    Always,
    Never,
}

impl From<OverwriteArg> for OverwriteMode {
    fn from(value: OverwriteArg) -> Self {
        match value {
            OverwriteArg::Ask => OverwriteMode::Ask,
            OverwriteArg::Always => OverwriteMode::Always,
            OverwriteArg::Never => OverwriteMode::Never,
        }
    }
}

#[derive(Debug, clap::Args)]
struct UnpackArgs {
    archive: PathBuf,
    #[arg(short = 'o', long = "output", default_value = ".")]
    output_dir: PathBuf,
    #[arg(long = "overwrite", value_enum, default_value_t = OverwriteArg::Always)]
    overwrite: OverwriteArg,
    #[arg(short = 'p', long = "password")]
    password: Option<String>,
}

#[derive(Debug, clap::Args)]
struct ListArgs {
    archive: PathBuf,
    #[arg(short = 'p', long = "password")]
    password: Option<String>,
    #[arg(long = "json", default_value_t = false)]
    json: bool,
}

#[derive(Debug, clap::Args)]
struct TestArgs {
    archive: PathBuf,
    #[arg(short = 'p', long = "password")]
    password: Option<String>,
}

#[derive(Debug, clap::Args)]
struct BenchArgs {
    source: PathBuf,
    #[arg(long = "format", short = 'f')]
    format: ArchiveFormat,
    #[arg(long = "level", alias = "mx")]
    level: Option<u32>,
    #[arg(long = "iterations", short = 'n', default_value_t = 3)]
    iterations: u32,
    #[arg(long = "warmup", default_value_t = 0)]
    warmup_iterations: u32,
    #[arg(long = "solid", default_value_t = false)]
    solid: bool,
    #[arg(short = 'p', long = "password")]
    password: Option<String>,
    #[arg(long = "json", default_value_t = false)]
    json: bool,
    #[arg(long = "out", value_name = "FILE")]
    out: Option<PathBuf>,
    #[arg(long = "csv", value_name = "FILE")]
    csv: Option<PathBuf>,
}

#[derive(Debug, Default)]
struct OutputFlags {
    list_as_json: bool,
    bench_as_json: bool,
    bench_out: Option<PathBuf>,
    bench_csv: Option<PathBuf>,
}

fn main() -> miette::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .without_time()
        .init();

    let cli = Cli::parse();
    let (request, output_flags) = to_request(cli).map_err(|err| miette!(err.to_string()))?;
    let report = k7z_core::run(request).map_err(|err| miette!(err.to_string()))?;
    print_report(&report, &output_flags).into_diagnostic()?;
    Ok(())
}

fn to_request(cli: Cli) -> Result<(TaskRequest, OutputFlags), K7zError> {
    match cli.command {
        Commands::Pack(args) => {
            let format = if let Some(raw) = args.format {
                raw.parse::<ArchiveFormat>()?
            } else {
                detect_format_from_path(&args.output).ok_or_else(|| {
                    K7zError::UnsupportedFormat(format!(
                        "cannot infer archive format from {}",
                        args.output.display()
                    ))
                })?
            };

            Ok((
                TaskRequest::Pack(PackRequest {
                    sources: args.sources,
                    output: args.output,
                    format,
                    level: args.level,
                    solid: args.solid,
                    password: args.password,
                }),
                OutputFlags::default(),
            ))
        }
        Commands::Unpack(args) => Ok((
            TaskRequest::Unpack(UnpackRequest {
                archive: args.archive,
                output_dir: args.output_dir,
                overwrite: args.overwrite.into(),
                password: args.password,
            }),
            OutputFlags::default(),
        )),
        Commands::List(args) => Ok((
            TaskRequest::List(ListRequest {
                archive: args.archive,
                password: args.password,
            }),
            OutputFlags {
                list_as_json: args.json,
                ..Default::default()
            },
        )),
        Commands::Test(args) => Ok((
            TaskRequest::Test(TestRequest {
                archive: args.archive,
                password: args.password,
            }),
            OutputFlags::default(),
        )),
        Commands::Bench(args) => Ok((
            TaskRequest::Bench(BenchRequest {
                source: args.source,
                format: args.format,
                level: args.level,
                iterations: args.iterations,
                warmup_iterations: args.warmup_iterations,
                solid: args.solid,
                password: args.password,
            }),
            OutputFlags {
                bench_as_json: args.json,
                bench_out: args.out,
                bench_csv: args.csv,
                ..Default::default()
            },
        )),
    }
}

fn print_report(report: &Report, output_flags: &OutputFlags) -> std::io::Result<()> {
    match report {
        Report::Pack(data) => {
            println!(
                "packed {} entries -> {} ({} bytes -> {} bytes)",
                data.entries,
                data.archive.display(),
                data.bytes_in,
                data.bytes_out
            );
        }
        Report::Unpack(data) => {
            println!(
                "unpacked {} entries into {} ({} bytes)",
                data.entries,
                data.output_dir.display(),
                data.bytes_out
            );
        }
        Report::List(data) => {
            if output_flags.list_as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(data).expect("json serialization should work")
                );
            } else {
                for entry in &data.entries {
                    let kind = if entry.is_dir { "dir " } else { "file" };
                    match entry.compressed_size {
                        Some(compressed) => {
                            println!(
                                "{kind}\t{}\t{} bytes\t{} bytes",
                                entry.path, entry.size, compressed
                            )
                        }
                        None => println!("{kind}\t{}\t{} bytes", entry.path, entry.size),
                    }
                }
            }
        }
        Report::Test(data) => {
            println!("test passed: checked {} entries", data.entries_checked);
        }
        Report::Bench(data) => {
            if let Some(path) = &output_flags.bench_out {
                write_json_file(path, data)?;
            }
            if let Some(path) = &output_flags.bench_csv {
                append_bench_csv(path, data)?;
            }
            if output_flags.bench_as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(data).expect("json serialization should work")
                );
            } else {
                let ratio = if data.total_input_bytes == 0 {
                    0.0
                } else {
                    data.total_output_bytes as f64 / data.total_input_bytes as f64
                };
                println!(
                    "bench done: {} iterations, {} ms, {:.2} MiB/s, ratio {:.3}",
                    data.iterations, data.elapsed_ms, data.throughput_mib_s, ratio
                );
                if data.warmup_iterations > 0 {
                    println!("warmup iterations: {}", data.warmup_iterations);
                }
                println!(
                    "input {} bytes -> output {} bytes",
                    data.total_input_bytes, data.total_output_bytes
                );
                if let Some(path) = &output_flags.bench_out {
                    println!("bench report written to {}", path.display());
                }
                if let Some(path) = &output_flags.bench_csv {
                    println!("bench csv appended to {}", path.display());
                }
            }
        }
    }
    Ok(())
}

fn write_json_file(path: &Path, value: &impl serde::Serialize) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let json = serde_json::to_string_pretty(value).expect("json serialization should work");
    std::fs::write(path, json)
}

fn append_bench_csv(path: &Path, data: &k7z_common::BenchReport) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let needs_header = !path.exists() || std::fs::metadata(path)?.len() == 0;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if needs_header {
        use std::io::Write as _;
        file.write_all(
            b"timestamp_unix_ms,format,iterations,warmup_iterations,total_input_bytes,total_output_bytes,elapsed_ms,throughput_mib_s,ratio\n",
        )?;
    }
    let ratio = if data.total_input_bytes == 0 {
        0.0
    } else {
        data.total_output_bytes as f64 / data.total_input_bytes as f64
    };
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    use std::io::Write as _;
    writeln!(
        file,
        "{},{},{},{},{},{},{},{:.6},{:.6}",
        ts_ms,
        data.format.as_str(),
        data.iterations,
        data.warmup_iterations,
        data.total_input_bytes,
        data.total_output_bytes,
        data.elapsed_ms,
        data.throughput_mib_s,
        ratio
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_json_file_creates_parent_dirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let output = dir.path().join("nested/report.json");
        write_json_file(&output, &serde_json::json!({"ok": true})).expect("write");
        let raw = std::fs::read_to_string(output).expect("read");
        assert!(raw.contains("\"ok\": true"));
    }

    #[test]
    fn append_bench_csv_writes_header_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let output = dir.path().join("nested/report.csv");
        let report = k7z_common::BenchReport {
            format: ArchiveFormat::Zip,
            iterations: 2,
            warmup_iterations: 1,
            total_input_bytes: 10,
            total_output_bytes: 8,
            elapsed_ms: 5,
            throughput_mib_s: 1.5,
        };
        append_bench_csv(&output, &report).expect("csv1");
        append_bench_csv(&output, &report).expect("csv2");
        let raw = std::fs::read_to_string(output).expect("read");
        let lines: Vec<_> = raw.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("timestamp_unix_ms,format"));
    }
}
