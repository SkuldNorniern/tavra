#![no_main]

use libfuzzer_sys::fuzz_target;
use tavra::envelope::{self, OpenMode};

// fixed keys so fuzzer can't reach past aead, but header, signature and
// decompress paths all still run
const KEY: [u8; 32] = [7; 32];
const PUBLIC: [u8; 32] = [9; 32];

fuzz_target!(|data: &[u8]| {
    for mode in [OpenMode::None, OpenMode::Key(&KEY)] {
        let _ = envelope::open(data, &mode, None);
        let _ = envelope::open(data, &mode, Some(&PUBLIC));
    }
});
