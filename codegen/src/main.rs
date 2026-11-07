//! Regenerates the `src/generated` modules from the vendored FatturaPA schemas.
//!
//! Run `cargo run -p codegen` from the workspace root after updating a schema,
//! and `cargo run -p codegen -- --check` to fail when a checked-in module is stale.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use prettyplease::unparse;
use quote::ToTokens;
use url::Url;
use xsd_parser::config::{Config, IdentQuadruple};
use xsd_parser::models::IdentType;
use xsd_parser::models::meta::CustomMeta;
use xsd_parser::pipeline::parser::Parser;
use xsd_parser::pipeline::parser::resolver::{ResolveRequest, ResolveResult, Resolver};
use xsd_parser::{
    exec_generator_with_ident_cache, exec_interpreter_with_ident_cache, exec_optimizer, exec_render,
};
use xsd_parser_types::misc::Namespace;

/// The `W3C XML-DSig` schema URL the schemas import, mapped onto the local copy.
const W3C_XMDSIG: &str =
    "http://www.w3.org/TR/2002/REC-xmldsig-core-20020212/xmldsig-core-schema.xsd";

/// The vendored schemas and the `src/generated` module each one feeds, by file name.
const SCHEMAS: &[(&str, &str)] = &[
    ("Schema_VFPR12_v1.2.3.xsd", "fpr12"),
    ("Schema_VFSM10v_1.0.2.xsd", "fsm10"),
    ("MessaggiTypes_v1.1.xsd", "sdi"),
];

/// What one run of the generator does.
#[derive(Clone, Copy)]
enum Mode {
    /// Writes the modules.
    Write,
    /// Fails when a checked-in module differs from a fresh one.
    Check,
    /// Prints the version of every vendored schema.
    SpecVersion,
}

fn main() -> ExitCode {
    let mode = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => Mode::Write,
        [flag] if flag == "--check" => Mode::Check,
        [flag] if flag == "--spec-version" => Mode::SpecVersion,
        _ => {
            eprintln!("usage: codegen [--check | --spec-version]");
            return ExitCode::from(2);
        }
    };
    match run(Path::new("."), mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("codegen: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the generator in `mode` on the workspace at `root`.
fn run(root: &Path, mode: Mode) -> Result<()> {
    let spec_dir =
        std::path::absolute(root.join("spec")).context("resolving the spec directory")?;
    let generated_dir = root.join("src/generated");
    match mode {
        Mode::SpecVersion => {
            for (schema, _) in SCHEMAS {
                let text = read(&spec_dir.join(schema), schema)?;
                println!("{schema} {}", spec_version(&text)?);
            }
            Ok(())
        }
        Mode::Check => {
            let mut stale = Vec::new();
            for (schema, module) in SCHEMAS {
                let path = generated_dir.join(format!("{module}.rs"));
                let current = std::fs::read_to_string(&path).unwrap_or_default();
                if current != fresh_module(&spec_dir, schema)? {
                    stale.push(schema.to_owned());
                }
            }
            if stale.is_empty() {
                Ok(())
            } else {
                anyhow::bail!(
                    "stale module(s) for {}, run `cargo run -p codegen`",
                    stale.join(", ")
                )
            }
        }
        Mode::Write => {
            std::fs::create_dir_all(&generated_dir)
                .with_context(|| format!("creating {}", generated_dir.display()))?;
            for (schema, module) in SCHEMAS {
                let path = generated_dir.join(format!("{module}.rs"));
                std::fs::write(&path, fresh_module(&spec_dir, schema)?)
                    .with_context(|| format!("writing {}", path.display()))?;
            }
            Ok(())
        }
    }
}

/// The formatted source of the generated module for `schema`, fresh from the
/// schemas at `spec_dir`.
fn fresh_module(spec_dir: &Path, schema: &str) -> Result<String> {
    let text = read(&spec_dir.join(schema), schema)?;
    let version = spec_version(&text)?;
    let body = generate(spec_dir, schema)?;
    Ok(format!(
        "//! Generated from `spec/{schema}`, version {version}.\n{body}"
    ))
}

/// Reads the file at `path`, naming it in the error.
fn read(path: &Path, schema: &str) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {schema} at {}", path.display()))
}

/// The `version` attribute of the schema root element in `text`.
fn spec_version(text: &str) -> Result<String> {
    let start = text
        .find("<xs:schema")
        .or_else(|| text.find("<xsd:schema"))
        .context("no schema root element")?;
    let end = text
        .get(start..)
        .and_then(|rest| rest.find('>').map(|offset| start + offset))
        .context("unclosed schema root element")?;
    attribute(&text[start..end], "version")
}

/// The value of the named attribute in a start-tag `tag`, or an error.
fn attribute(tag: &str, name: &str) -> Result<String> {
    let needle = format!("{name}=");
    let start = tag
        .find(&needle)
        .with_context(|| format!("no {name} attribute"))?
        + needle.len();
    let rest = &tag[start..];
    let (quote, body) = if let Some(body) = rest.strip_prefix('"') {
        ('"', body)
    } else if let Some(body) = rest.strip_prefix('\'') {
        ('\'', body)
    } else {
        anyhow::bail!("unquoted {name} attribute")
    };
    let end = body
        .find(quote)
        .with_context(|| format!("unterminated {name} attribute"))?;
    Ok(body[..end].to_owned())
}

/// Drives the `xsd-parser` pipeline for one schema and returns the formatted module.
fn generate(spec_dir: &Path, schema: &str) -> Result<String> {
    let schema_path = spec_dir.join(schema);
    let mut parser: Parser<CatalogResolver> = Parser::new().with_resolver(CatalogResolver {
        spec_dir: spec_dir.to_path_buf(),
    });
    parser = parser
        .resolve_includes(true)
        .generate_prefixes(true)
        .alternative_prefixes(true)
        .with_default_namespaces()
        .add_schema_from_file(&schema_path)
        .with_context(|| format!("parsing {schema}"))?;
    let schemas = parser.finish();

    let config = Config::default()
        .with_quick_xml()
        .with_derive(["Debug", "Clone", "PartialEq", "Eq", "Hash"])
        .with_type(
            IdentQuadruple::new("decimal", IdentType::Type).with_ns(Namespace::XS),
            CustomMeta::new("Amount").include_from("crate::amount::Amount"),
        )
        .with_type(
            IdentQuadruple::new("date", IdentType::Type).with_ns(Namespace::XS),
            CustomMeta::new("Date").include_from("crate::date::Date"),
        )
        .with_type(
            IdentQuadruple::new("dateTime", IdentType::Type).with_ns(Namespace::XS),
            CustomMeta::new("DateTime").include_from("crate::date::DateTime"),
        );

    let (meta_types, ident_cache) = exec_interpreter_with_ident_cache(config.interpreter, &schemas)
        .with_context(|| format!("interpreting {schema}"))?;
    let meta_types = exec_optimizer(config.optimizer, meta_types)
        .with_context(|| format!("optimizing {schema}"))?;
    let data_types = exec_generator_with_ident_cache(
        config.generator,
        &schemas,
        Some(&ident_cache),
        &meta_types,
    )
    .with_context(|| format!("generating {schema}"))?;
    let module =
        exec_render(config.renderer, &data_types).with_context(|| format!("rendering {schema}"))?;
    let file = syn::parse2(module.to_token_stream())
        .with_context(|| format!("parsing the generated source of {schema}"))?;
    Ok(unparse(&file))
}

/// Resolver that mirrors `spec/catalog.xml`: the W3C XML-DSig URL maps onto the
/// local `spec/xmldsig-core-schema.xsd`, everything else resolves as a file.
#[derive(Debug)]
struct CatalogResolver {
    /// The `spec` directory the schemas resolve from.
    spec_dir: PathBuf,
}

impl Resolver for CatalogResolver {
    type Buffer = BufReader<File>;
    type Error = std::io::Error;

    fn resolve(&mut self, req: &ResolveRequest) -> ResolveResult<Self> {
        let path = if req.requested_location == W3C_XMDSIG {
            self.spec_dir.join("xmldsig-core-schema.xsd")
        } else {
            let url = if let Ok(parsed) = Url::parse(&req.requested_location) {
                parsed
            } else {
                let current = req.current_location.as_ref().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "relative location without current location",
                    )
                })?;
                current.join(&req.requested_location).map_err(|error| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string())
                })?
            };
            url.to_file_path().map_err(|()| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a file URL")
            })?
        };

        let file = BufReader::new(File::open(&path)?);
        let url = Url::from_file_path(&path).map_err(|()| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a file path")
        })?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned);
        Ok(Some((name, url, file)))
    }
}
