#![no_main]

use std::path::Path;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let path = path_from_bytes(data);
    let _ = k7z_common::detect_format_from_path(Path::new(&path));
});

fn path_from_bytes(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().min(256) + 8);
    for byte in input.iter().copied().take(256) {
        let ch = match byte % 10 {
            0 => '.',
            1 => '/',
            2 => '-',
            _ => (b'a' + (byte % 26)) as char,
        };
        out.push(ch);
    }
    out
}
