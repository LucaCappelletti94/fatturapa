//! Byte-level verification of `XAdES` signed documents, for panics. The
//! input runs through the tree reader, the canonicalization of every
//! reference and of the `SignedInfo`, and the signer selection.

#![no_main]

use fatturapa::signed;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let _ = signed::verify_xades(input, &[], None);
});
