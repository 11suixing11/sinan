#![forbid(unsafe_code)]

// Reuse the deliberately public test key; this example cannot accept another key.
#[path = "../tests/support/release.rs"]
mod fixture;

use std::io::{Read, Write};

fn main() -> std::io::Result<()> {
    let maximum = sinan_protocol::release::MAX_CHECKSUMS_BYTES;
    let mut checksums = Vec::new();
    std::io::stdin()
        .take(maximum as u64 + 1)
        .read_to_end(&mut checksums)?;
    if checksums.len() > maximum || std::env::args_os().len() != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "TEST_ONLY signer accepts bounded checksums on stdin and no arguments",
        ));
    }
    std::io::stdout().write_all(fixture::sign(&checksums).as_bytes())
}
