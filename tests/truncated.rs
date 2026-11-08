//! A truncated document is refused, wherever the input ends.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing step"
)]

#[path = "support/mod.rs"]
mod support;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use fatturapa::{Document, NotificaDecorrenzaTermini};

/// How long one read may take before the test reports it as a hang.
const BOUND: Duration = Duration::from_secs(10);

/// The official `DT` sample cut right after the first occurrence of `marker`.
fn dt_cut_after(marker: &str) -> Vec<u8> {
    let sample = support::sample_bytes("IT01234567890_11111_DT_001.xml");
    let at = sample
        .windows(marker.len())
        .position(|window| window == marker.as_bytes())
        .expect("the marker occurs in the sample")
        + marker.len();
    sample[..at].to_vec()
}

/// Reads `input` on its own thread and checks it fails within [`BOUND`].
fn refused_in_time(input: Vec<u8>) {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let failed = NotificaDecorrenzaTermini::from_xml_slice(&input).is_err();
        let _ = sender.send(failed);
    });
    let failed = receiver
        .recv_timeout(BOUND)
        .expect("the read returns instead of hanging");
    assert!(failed, "a truncated document is refused");
}

/// Input that ends inside `ds:SignedInfo`, a plain sequence.
#[test]
fn truncated_inside_signed_info_is_refused() {
    refused_in_time(dt_cut_after("<ds:SignedInfo>"));
}

/// Input that ends inside `ds:KeyInfo`, a mixed choice.
#[test]
fn truncated_inside_key_info_is_refused() {
    refused_in_time(dt_cut_after("<ds:KeyInfo Id=\"KeyInfoId\">"));
}

/// Input that ends inside `ds:Object`, a mixed sequence of `xs:any`.
#[test]
fn truncated_inside_object_is_refused() {
    refused_in_time(dt_cut_after("<ds:Object>"));
}
