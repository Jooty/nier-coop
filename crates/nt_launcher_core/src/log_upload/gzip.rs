use std::io::{self, Read, Write};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

pub fn gzip_text(text: &str) -> io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(text.as_bytes())?;
    encoder.finish()
}

pub fn gunzip_text(bytes: &[u8]) -> io::Result<String> {
    let mut text = String::new();
    GzDecoder::new(bytes).read_to_string(&mut text)?;
    Ok(text)
}
