//! Byte-level reading of the ordinary invoice, for panics.

#![no_main]

use fatturapa::Document;
use fatturapa::FatturaElettronica;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let _ = FatturaElettronica::from_xml_slice(input);
});
