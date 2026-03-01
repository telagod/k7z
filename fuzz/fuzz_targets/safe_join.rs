#![no_main]

use std::path::PathBuf;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let split = data.iter().position(|b| *b == 0).unwrap_or(data.len());
    let (base_raw, rest) = data.split_at(split);
    let rel_raw = if split < data.len() { &rest[1..] } else { rest };

    let base = PathBuf::from(path_from_bytes(base_raw));
    let rel = PathBuf::from(path_from_bytes(rel_raw));

    let _ = k7z_common::safe_join(&base, &rel);
});

fn path_from_bytes(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().min(256));
    for byte in input.iter().copied().take(256) {
        let ch = match byte % 12 {
            0 => '.',
            1 => '/',
            2 => '_',
            3 => '-',
            _ => (b'a' + (byte % 26)) as char,
        };
        out.push(ch);
    }
    out
}
