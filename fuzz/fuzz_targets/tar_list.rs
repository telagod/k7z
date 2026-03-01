#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = k7z_format_tar::list_from_reader(Cursor::new(data));
});
