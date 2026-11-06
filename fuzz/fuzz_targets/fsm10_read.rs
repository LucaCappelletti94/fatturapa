//! Byte-level reading of the simplified invoice, for panics.

#![no_main]

use fatturapa::Document;
use fatturapa::FatturaElettronicaSemplificata;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let _ = FatturaElettronicaSemplificata::from_xml_slice(input);
});
