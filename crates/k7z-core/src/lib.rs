use k7z_common::{
    ArchiveFormat, BenchReport, BenchRequest, K7zError, ListReport, PackReport, PackRequest,
    Report, Result, TaskRequest, TestReport, TestRequest, UnpackReport, UnpackRequest,
    detect_format_from_path,
};
use std::time::Instant;

pub fn run(request: TaskRequest) -> Result<Report> {
    match request {
        TaskRequest::Pack(req) => Ok(Report::Pack(pack(req)?)),
        TaskRequest::Unpack(req) => Ok(Report::Unpack(unpack(req)?)),
        TaskRequest::List(req) => Ok(Report::List(list(req)?)),
        TaskRequest::Test(req) => Ok(Report::Test(test(req)?)),
        TaskRequest::Bench(req) => Ok(Report::Bench(bench(req)?)),
    }
}

pub fn pack(req: PackRequest) -> Result<PackReport> {
    match req.format {
        ArchiveFormat::SevenZ => k7z_format_7z::pack(&req),
        ArchiveFormat::Zip => k7z_format_zip::pack(&req),
        ArchiveFormat::Tar
        | ArchiveFormat::TarGz
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarZst => k7z_format_tar::pack(&req),
        ArchiveFormat::Zst => k7z_format_zstd::pack(&req),
    }
}

pub fn unpack(req: UnpackRequest) -> Result<UnpackReport> {
    let format = detect_archive_format(&req.archive)?;
    match format {
        ArchiveFormat::SevenZ => k7z_format_7z::unpack(&req),
        ArchiveFormat::Zip => k7z_format_zip::unpack(&req),
        ArchiveFormat::Tar
        | ArchiveFormat::TarGz
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarZst => k7z_format_tar::unpack(&req, format),
        ArchiveFormat::Zst => k7z_format_zstd::unpack(&req),
    }
}

pub fn list(req: k7z_common::ListRequest) -> Result<ListReport> {
    let format = detect_archive_format(&req.archive)?;
    match format {
        ArchiveFormat::SevenZ => k7z_format_7z::list(&req),
        ArchiveFormat::Zip => k7z_format_zip::list(&req),
        ArchiveFormat::Tar
        | ArchiveFormat::TarGz
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarZst => k7z_format_tar::list(&req, format),
        ArchiveFormat::Zst => k7z_format_zstd::list(&req),
    }
}

pub fn test(req: TestRequest) -> Result<TestReport> {
    let format = detect_archive_format(&req.archive)?;
    match format {
        ArchiveFormat::SevenZ => k7z_format_7z::test(&req),
        ArchiveFormat::Zip => k7z_format_zip::test(&req),
        ArchiveFormat::Tar
        | ArchiveFormat::TarGz
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarZst => k7z_format_tar::test(&req, format),
        ArchiveFormat::Zst => k7z_format_zstd::test(&req),
    }
}

pub fn bench(req: BenchRequest) -> Result<BenchReport> {
    if !req.source.exists() {
        return Err(K7zError::InvalidInput(format!(
            "source does not exist: {}",
            req.source.display()
        )));
    }
    if req.iterations == 0 {
        return Err(K7zError::InvalidInput(
            "iterations must be greater than 0".to_string(),
        ));
    }

    let scratch = tempfile::tempdir()?;
    run_bench_pack_loop(&req, &scratch.path().join("warmup"), req.warmup_iterations)?;

    let start = Instant::now();
    let (total_input_bytes, total_output_bytes) =
        run_bench_pack_loop(&req, &scratch.path().join("measure"), req.iterations)?;

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_millis().max(1);
    let throughput_mib_s =
        (total_input_bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64().max(0.001);

    Ok(BenchReport {
        format: req.format,
        iterations: req.iterations,
        warmup_iterations: req.warmup_iterations,
        total_input_bytes,
        total_output_bytes,
        elapsed_ms,
        throughput_mib_s,
    })
}

fn run_bench_pack_loop(
    req: &BenchRequest,
    output_root: &std::path::Path,
    iterations: u32,
) -> Result<(u64, u64)> {
    std::fs::create_dir_all(output_root)?;
    let mut total_input_bytes = 0_u64;
    let mut total_output_bytes = 0_u64;
    for i in 0..iterations {
        let archive_path =
            output_root.join(format!("bench-{}.{}", i + 1, extension_for(req.format)));
        let report = pack(PackRequest {
            sources: vec![req.source.clone()],
            output: archive_path,
            format: req.format,
            level: req.level,
            solid: req.solid,
            password: req.password.clone(),
        })?;
        total_input_bytes = total_input_bytes.saturating_add(report.bytes_in);
        total_output_bytes = total_output_bytes.saturating_add(report.bytes_out);
    }
    Ok((total_input_bytes, total_output_bytes))
}

fn detect_archive_format(path: &std::path::Path) -> Result<ArchiveFormat> {
    detect_format_from_path(path).ok_or_else(|| {
        K7zError::UnsupportedFormat(format!(
            "cannot infer archive format from path {}",
            path.display()
        ))
    })
}

fn extension_for(format: ArchiveFormat) -> &'static str {
    match format {
        ArchiveFormat::SevenZ => "7z",
        ArchiveFormat::Zip => "zip",
        ArchiveFormat::Tar => "tar",
        ArchiveFormat::TarGz => "tar.gz",
        ArchiveFormat::TarXz => "tar.xz",
        ArchiveFormat::TarZst => "tar.zst",
        ArchiveFormat::Zst => "zst",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn unknown_extension_is_rejected() {
        let path = std::path::Path::new("/tmp/archive.unknown");
        assert!(detect_archive_format(path).is_err());
    }

    #[test]
    fn bench_runs_zip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("a.txt");
        fs::write(&src, b"hello bench").expect("write");

        let report = bench(BenchRequest {
            source: src,
            format: ArchiveFormat::Zip,
            level: Some(6),
            iterations: 2,
            warmup_iterations: 1,
            solid: false,
            password: None,
        })
        .expect("bench");

        assert_eq!(report.format, ArchiveFormat::Zip);
        assert_eq!(report.iterations, 2);
        assert_eq!(report.warmup_iterations, 1);
        assert!(report.total_input_bytes > 0);
        assert!(report.total_output_bytes > 0);
    }
}
