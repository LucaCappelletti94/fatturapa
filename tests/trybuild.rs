//! The `trybuild` compile-fail tests, one per illegal state of the typed
//! construction API. Each case is a program that the typed layer refuses
//! at compile time. The `.stderr` snapshots are generated with the stable
//! toolchain, so the test runs only on stable and skips elsewhere, where
//! the compiler renders the diagnostics differently.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing case"
)]

use trybuild::TestCases;

fn stable_rustc() -> bool {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .expect("the rustc of the active toolchain runs");
    let version = String::from_utf8_lossy(&output.stdout);
    version.starts_with("rustc 1.") && !version.contains("nightly") && !version.contains("-beta")
}

#[test]
fn compile_fail() {
    if !stable_rustc() {
        eprintln!("skipped: the compile-fail snapshots match the stable toolchain");
        return;
    }
    let t = TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
