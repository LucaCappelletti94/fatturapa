//! Byte-level unwrapping of signed files, for panics. The unwrapped content
//! is then read as an invoice, in either form, so the parser sees the `XML`
//! the signer enclosed.

#![no_main]

use fatturapa::Document;
use fatturapa::signed;
use fatturapa::{FatturaElettronica, FatturaElettronicaSemplificata};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let Ok(content) = signed::unwrap(input) else {
        return;
    };
    let _ = FatturaElettronica::from_xml_slice(&content);
    if content[..content.len().min(600)]
        .windows(32)
        .any(|window| window.iter().zip("FatturaElettronicaSemplificata".as_bytes()).all(|(a, b)| a == b))
    {
        let _ = FatturaElettronicaSemplificata::from_xml_slice(&content);
    }
});
