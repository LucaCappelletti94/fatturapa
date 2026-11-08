//! The official `Rappresentazione tabellare` tables checked against the doc
//! table of `src/validate.rs`.
//!
//! Each field row of the two tables in `spec/` lists, in the last column,
//! the `SDI` check codes the exchange can report for that field. The test
//! extracts every (code, field id) pair from both tables and checks the
//! `validate` doc table in both directions. Every code a table lists must
//! have a non-empty cell for that format in the doc table, which says the
//! check is implemented, guaranteed by construction, rejected by the
//! generated reader, or out of reach with a reason. Every code the doc
//! table marks implemented for a format must appear in that format's
//! table, or in the explicit list below, which names the official source
//! that defines the check.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use calamine::{Data, Reader, Xlsx, open_workbook};

/// The `SDI` check codes the doc table marks implemented that neither table
/// assigns to a field, with the official source that defines them.
const OFF_TABLE: &[(&str, &str, &str)] = &[
    (
        "00330",
        "ordinary",
        "Allegato A 1.9.1 defines the PECDestinatario check for the ordinary invoice (field 1.1.6), and no table row lists the code",
    ),
    (
        "00330",
        "simplified",
        "Allegato A 1.9.1 defines the PECDestinatario check for the simplified invoice (field 1.1.5), and no table row lists the code",
    ),
    (
        "00409",
        "ordinary",
        "Allegato A 1.9.1 and the Elenco Controlli define the duplicate invoice in the lot check, and no table row lists the code",
    ),
    (
        "00409",
        "simplified",
        "Allegato A 1.9.1 and the Elenco Controlli define the duplicate invoice in the lot check, and no table row lists the code",
    ),
];

/// The cell texts that account for a code in the doc table.
const ACCEPTED: &[&str] = &[
    "implemented",
    "out of reach",
    "guaranteed by construction",
    "the generated reader rejects it",
];

/// One row of the `validate` doc table.
struct DocRow {
    /// The `SDI` check code.
    code: String,
    /// The ordinary invoice cell.
    ordinary: String,
    /// The simplified invoice cell.
    simplified: String,
}

/// The rows of the doc table in `src/validate.rs`.
fn doc_rows() -> Vec<DocRow> {
    let source = include_str!("../src/validate.rs");
    let mut rows = Vec::new();
    let mut in_table = false;
    for line in source.lines() {
        let Some(line) = line.strip_prefix("//! |") else {
            if in_table {
                break;
            }
            continue;
        };
        let cells: Vec<&str> = line
            .trim_end_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        let separator = !cells.is_empty()
            && cells
                .iter()
                .all(|c| c.is_empty() || c.chars().all(|c| c == '-'));
        if separator {
            continue;
        }
        if cells.len() != 4 {
            if in_table {
                break;
            }
            continue;
        }
        if cells[0] == "Code" {
            in_table = true;
            continue;
        }
        if cells[0].len() != 5 || !cells[0].chars().all(|c| c.is_ascii_digit()) {
            if in_table {
                break;
            }
            continue;
        }
        rows.push(DocRow {
            code: cells[0].to_owned(),
            ordinary: cells[2].to_owned(),
            simplified: cells[3].to_owned(),
        });
    }
    rows
}

/// The dotted field id at the start of a cell that names a field, if any.
fn field_id(cell: &str) -> Option<&str> {
    let token = cell.split_whitespace().next()?;
    if !cell.contains('<') {
        return None;
    }
    if token.is_empty()
        || token.starts_with('.')
        || token.ends_with('.')
        || !token.contains('.')
        || !token.chars().all(|c| c.is_ascii_digit() || c == '.')
    {
        return None;
    }
    Some(token)
}

/// The `SDI` check code at the start of a line of the code column, if any.
fn code_of(line: &str) -> Option<String> {
    let mut chars = line.trim_start().chars();
    let code: String = chars.by_ref().take(5).collect();
    (code.len() == 5 && code.chars().all(|c| c.is_ascii_digit()) && chars.next() == Some(':'))
        .then_some(code)
}

/// The (code, field id) pairs of one table, in sheet order.
fn table_pairs(path: &Path) -> Vec<(String, String)> {
    let mut book =
        open_workbook::<Xlsx<BufReader<File>>, _>(path).expect("the vendored table opens");
    let range = book
        .worksheet_range_at(0)
        .expect("the table has a sheet")
        .expect("the first sheet parses");
    let rows: Vec<Vec<&str>> = range
        .rows()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    Data::String(text) => text.as_str(),
                    _ => "",
                })
                .collect()
        })
        .collect();
    let header = rows
        .iter()
        .position(|r| r.iter().any(|c| c.starts_with("Codice e descrizione")))
        .expect("the header row names the code column");
    let codes = rows[header]
        .iter()
        .position(|c| c.starts_with("Codice e descrizione"))
        .expect("the header has the code column");
    let mut pairs = Vec::new();
    for row in rows.iter().skip(header + 1) {
        let Some(id) = row.iter().find_map(|c| field_id(c)) else {
            continue;
        };
        for line in row[codes].lines() {
            if let Some(code) = code_of(line) {
                pairs.push((code, id.to_owned()));
            }
        }
    }
    pairs
}

/// The distinct codes one table lists.
fn table_codes(pairs: &[(String, String)]) -> BTreeSet<&str> {
    pairs.iter().map(|(code, _)| code.as_str()).collect()
}

/// The ordinary table lists every code the doc table accounts for.
#[test]
fn ordinary_table_codes_match_the_doc_table() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("spec/RappresentazioneTabellareFattOrdinaria-1.xlsx");
    let pairs = table_pairs(&path);
    let rows = doc_rows();
    for (code, field) in &pairs {
        let row = rows.iter().find(|r| r.code == *code).unwrap_or_else(|| {
            panic!("code {code} from field {field} is missing from the validate doc table")
        });
        assert!(
            ACCEPTED.iter().any(|p| row.ordinary.starts_with(p)),
            "code {code} from field {field} has no ordinary invoice cell in the validate doc table: {}",
            row.ordinary
        );
    }
    assert_eq!(
        table_codes(&pairs).len(),
        51,
        "the ordinary table lists 51 distinct codes today"
    );
}

/// The simplified table lists every code the doc table accounts for.
#[test]
fn simplified_table_codes_match_the_doc_table() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("spec/RappresentazioneTabellareFattSemplificata1.xlsx");
    let pairs = table_pairs(&path);
    let rows = doc_rows();
    for (code, field) in &pairs {
        let row = rows.iter().find(|r| r.code == *code).unwrap_or_else(|| {
            panic!("code {code} from field {field} is missing from the validate doc table")
        });
        assert!(
            ACCEPTED.iter().any(|p| row.simplified.starts_with(p)),
            "code {code} from field {field} has no simplified invoice cell in the validate doc table: {}",
            row.simplified
        );
    }
    assert_eq!(
        table_codes(&pairs).len(),
        25,
        "the simplified table lists 25 distinct codes today"
    );
}

/// Every implemented code the doc table names appears in a table or in
/// `OFF_TABLE`.
#[test]
fn implemented_codes_appear_in_the_tables() {
    let ordinary = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("spec/RappresentazioneTabellareFattOrdinaria-1.xlsx");
    let simplified = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("spec/RappresentazioneTabellareFattSemplificata1.xlsx");
    let ordinary_pairs = table_pairs(&ordinary);
    let ordinary_codes = table_codes(&ordinary_pairs);
    let simplified_pairs = table_pairs(&simplified);
    let simplified_codes = table_codes(&simplified_pairs);
    for row in doc_rows() {
        if row.ordinary.starts_with("implemented") {
            let listed = ordinary_codes.contains(row.code.as_str())
                || OFF_TABLE
                    .iter()
                    .any(|(code, format, _)| *code == row.code && *format == "ordinary");
            assert!(
                listed,
                "code {} is implemented for the ordinary invoice but appears in neither the table nor the official source list",
                row.code
            );
        }
        if row.simplified.starts_with("implemented") {
            let listed = simplified_codes.contains(row.code.as_str())
                || OFF_TABLE
                    .iter()
                    .any(|(code, format, _)| *code == row.code && *format == "simplified");
            assert!(
                listed,
                "code {} is implemented for the simplified invoice but appears in neither the table nor the official source list",
                row.code
            );
        }
    }
}

/// The doc table has the rows it has today.
#[test]
fn the_doc_table_has_its_rows() {
    assert_eq!(
        doc_rows().len(),
        69,
        "the validate doc table has 69 rows today"
    );
}
