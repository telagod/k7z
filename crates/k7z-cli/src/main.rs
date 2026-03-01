use std::path::PathBuf;

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
    #[arg(long = "solid", default_value_t = false)]
    solid: bool,
    #[arg(short = 'p', long = "password")]
    password: Option<String>,
}

fn main() -> miette::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .without_time()
        .init();

    let cli = Cli::parse();
    let (request, list_as_json) = to_request(cli).map_err(|err| miette!(err.to_string()))?;
    let report = k7z_core::run(request).map_err(|err| miette!(err.to_string()))?;
    print_report(&report, list_as_json).into_diagnostic()?;
    Ok(())
}

fn to_request(cli: Cli) -> Result<(TaskRequest, bool), K7zError> {
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
                false,
            ))
        }
        Commands::Unpack(args) => Ok((
            TaskRequest::Unpack(UnpackRequest {
                archive: args.archive,
                output_dir: args.output_dir,
                overwrite: args.overwrite.into(),
                password: args.password,
            }),
            false,
        )),
        Commands::List(args) => Ok((
            TaskRequest::List(ListRequest {
                archive: args.archive,
                password: args.password,
            }),
            args.json,
        )),
        Commands::Test(args) => Ok((
            TaskRequest::Test(TestRequest {
                archive: args.archive,
                password: args.password,
            }),
            false,
        )),
        Commands::Bench(args) => Ok((
            TaskRequest::Bench(BenchRequest {
                source: args.source,
                format: args.format,
                level: args.level,
                iterations: args.iterations,
                solid: args.solid,
                password: args.password,
            }),
            false,
        )),
    }
}

fn print_report(report: &Report, list_as_json: bool) -> std::io::Result<()> {
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
            if list_as_json {
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
            let ratio = if data.total_input_bytes == 0 {
                0.0
            } else {
                data.total_output_bytes as f64 / data.total_input_bytes as f64
            };
            println!(
                "bench done: {} iterations, {} ms, {:.2} MiB/s, ratio {:.3}",
                data.iterations, data.elapsed_ms, data.throughput_mib_s, ratio
            );
            println!(
                "input {} bytes -> output {} bytes",
                data.total_input_bytes, data.total_output_bytes
            );
        }
    }
    Ok(())
}
