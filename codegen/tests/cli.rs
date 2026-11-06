//! The generator as continuous integration and maintainers run it, on a copy of the workspace.

#![expect(clippy::unwrap_used, reason = "a test panics on any unexpected value")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The schema file names that `codegen` reads from `spec`.
const SCHEMAS: &[&str] = &[
    "Schema_VFPR12_v1.2.3.xsd",
    "Schema_VFSM10v_1.0.2.xsd",
    "MessaggiTypes_v1.1.xsd",
];

/// The module each schema feeds, in the order of `SCHEMAS`.
const MODULES: &[&str] = &["fpr12", "fsm10", "sdi"];

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// A temporary workspace holding the vendored schemas and, unless `modules` is
/// `None`, the generated modules to check.
fn copy(modules: Option<&[String]>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("spec")).unwrap();
    std::fs::create_dir_all(dir.path().join("src/generated")).unwrap();
    for schema in SCHEMAS {
        std::fs::copy(
            workspace().join("spec").join(schema),
            dir.path().join("spec").join(schema),
        )
        .unwrap();
    }
    std::fs::copy(
        workspace().join("spec/xmldsig-core-schema.xsd"),
        dir.path().join("spec/xmldsig-core-schema.xsd"),
    )
    .unwrap();
    if let Some(modules) = modules {
        for (module, source) in MODULES.iter().zip(modules) {
            std::fs::write(
                dir.path()
                    .join("src/generated")
                    .join(format!("{module}.rs")),
                source,
            )
            .unwrap();
        }
    }
    dir
}

fn codegen(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codegen"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn checked_in() -> Vec<String> {
    MODULES
        .iter()
        .map(|module| {
            std::fs::read_to_string(
                workspace()
                    .join("src/generated")
                    .join(format!("{module}.rs")),
            )
            .unwrap()
        })
        .collect()
}

#[test]
fn generation_reproduces_the_checked_in_modules() {
    let dir = copy(None);
    let output = codegen(dir.path(), &[]);
    assert!(output.status.success(), "{output:?}");
    let fresh = MODULES
        .iter()
        .map(|module| {
            std::fs::read_to_string(
                dir.path()
                    .join("src/generated")
                    .join(format!("{module}.rs")),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(fresh, checked_in());
}

#[test]
fn check_accepts_a_fresh_module_set_and_rejects_a_stale_one() {
    let current = checked_in();
    let fresh = copy(Some(&current));
    let output = codegen(fresh.path(), &["--check"]);
    assert!(output.status.success(), "{output:?}");

    let mut stale = current;
    stale[0] = stale[0].replace("FatturaElettronicaType", "FatturaElettronicaTypeStale");
    let stale_dir = copy(Some(&stale));
    let output = codegen(stale_dir.path(), &["--check"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("stale"),
        "{output:?}"
    );
    assert_eq!(
        std::fs::read_to_string(stale_dir.path().join("src/generated/fpr12.rs")).unwrap(),
        stale[0],
        "--check must not rewrite the modules"
    );
}

#[test]
fn the_spec_versions_are_printed_from_the_vendored_schemas() {
    let dir = copy(None);
    let output = codegen(dir.path(), &["--spec-version"]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    for (schema, module) in SCHEMAS.iter().zip(MODULES) {
        let line = stdout
            .lines()
            .find(|line| line.starts_with(schema))
            .unwrap_or_else(|| panic!("no line for {schema} in {stdout}"));
        let version = line.split_whitespace().last().unwrap().to_owned();
        assert!(
            checked_in()
                .iter()
                .zip(MODULES)
                .find(|(_, m)| *m == module)
                .unwrap()
                .0
                .contains(&format!("version {version}")),
            "{line}"
        );
    }
}

#[test]
fn unknown_arguments_are_refused() {
    let dir = copy(None);
    let output = codegen(dir.path(), &["--chek"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("usage"),
        "{output:?}"
    );
}
