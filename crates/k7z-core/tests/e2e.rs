use std::fs;

use k7z_common::{
    ArchiveFormat, ListRequest, OverwriteMode, PackRequest, TaskRequest, TestRequest, UnpackRequest,
};

#[test]
fn roundtrip_zip_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("sample.txt");
    fs::write(&src, b"zip-content").expect("write");
    let archive = dir.path().join("sample.zip");
    let out = dir.path().join("unzipped");

    let pack = k7z_core::run(TaskRequest::Pack(PackRequest {
        sources: vec![src.clone()],
        output: archive.clone(),
        format: ArchiveFormat::Zip,
        level: Some(6),
        solid: false,
        password: None,
    }));
    assert!(pack.is_ok());

    let list = k7z_core::run(TaskRequest::List(ListRequest {
        archive: archive.clone(),
        password: None,
    }));
    assert!(list.is_ok());

    let test = k7z_core::run(TaskRequest::Test(TestRequest {
        archive: archive.clone(),
        password: None,
    }));
    assert!(test.is_ok());

    let unpack = k7z_core::run(TaskRequest::Unpack(UnpackRequest {
        archive,
        output_dir: out.clone(),
        overwrite: OverwriteMode::Always,
        password: None,
    }));
    assert!(unpack.is_ok());
    assert_eq!(
        fs::read(out.join("sample.txt")).expect("read back"),
        b"zip-content"
    );
}

#[test]
fn roundtrip_7z_encrypted_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("plain.txt");
    fs::write(&src, b"sevenz-content").expect("write");
    let archive = dir.path().join("plain.7z");
    let out = dir.path().join("out7z");

    let pack = k7z_core::run(TaskRequest::Pack(PackRequest {
        sources: vec![src.clone()],
        output: archive.clone(),
        format: ArchiveFormat::SevenZ,
        level: Some(6),
        solid: true,
        password: Some("secret".to_string()),
    }));
    assert!(pack.is_ok());

    let test = k7z_core::run(TaskRequest::Test(TestRequest {
        archive: archive.clone(),
        password: Some("secret".to_string()),
    }));
    assert!(test.is_ok());

    let unpack = k7z_core::run(TaskRequest::Unpack(UnpackRequest {
        archive,
        output_dir: out.clone(),
        overwrite: OverwriteMode::Always,
        password: Some("secret".to_string()),
    }));
    assert!(unpack.is_ok());
    assert_eq!(
        fs::read(out.join("plain.txt")).expect("read back"),
        b"sevenz-content"
    );
}

#[test]
fn roundtrip_tar_zst_and_zst_stream() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("data.bin");
    fs::write(&src, b"tarzst-content").expect("write");

    let tar_zst = dir.path().join("data.tar.zst");
    let out_tar = dir.path().join("out-tar");
    let zst = dir.path().join("data.bin.zst");
    let out_zst = dir.path().join("out-zst");

    assert!(
        k7z_core::run(TaskRequest::Pack(PackRequest {
            sources: vec![src.clone()],
            output: tar_zst.clone(),
            format: ArchiveFormat::TarZst,
            level: Some(3),
            solid: false,
            password: None,
        }))
        .is_ok()
    );
    assert!(
        k7z_core::run(TaskRequest::Unpack(UnpackRequest {
            archive: tar_zst,
            output_dir: out_tar.clone(),
            overwrite: OverwriteMode::Always,
            password: None,
        }))
        .is_ok()
    );
    assert_eq!(
        fs::read(out_tar.join("data.bin")).expect("tar read"),
        b"tarzst-content"
    );

    assert!(
        k7z_core::run(TaskRequest::Pack(PackRequest {
            sources: vec![src],
            output: zst.clone(),
            format: ArchiveFormat::Zst,
            level: Some(3),
            solid: false,
            password: None,
        }))
        .is_ok()
    );
    assert!(
        k7z_core::run(TaskRequest::Unpack(UnpackRequest {
            archive: zst,
            output_dir: out_zst.clone(),
            overwrite: OverwriteMode::Always,
            password: None,
        }))
        .is_ok()
    );
    assert_eq!(
        fs::read(out_zst.join("data.bin")).expect("zst read"),
        b"tarzst-content"
    );
}
