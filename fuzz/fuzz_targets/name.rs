//! Byte-level parsing of the `SDI` invoice and message file names, for panics.

#![no_main]

use std::str;

use fatturapa::{InvoiceName, MessageName};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    // The name parsers take `&str`, so only valid UTF-8 reaches them.
    let Ok(text) = str::from_utf8(input) else {
        return;
    };
    let _ = text.parse::<InvoiceName>();
    let _ = text.parse::<MessageName>();
});
