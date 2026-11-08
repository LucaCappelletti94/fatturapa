# fatturapa

[![CI](https://github.com/LucaCappelletti94/fatturapa/actions/workflows/ci.yml/badge.svg)](https://github.com/LucaCappelletti94/fatturapa/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/LucaCappelletti94/fatturapa/graph/badge.svg)](https://codecov.io/gh/LucaCappelletti94/fatturapa)
[![crates.io](https://img.shields.io/crates/v/fatturapa.svg)](https://crates.io/crates/fatturapa)
[![docs.rs](https://docs.rs/fatturapa/badge.svg)](https://docs.rs/fatturapa)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/LucaCappelletti94/fatturapa/blob/main/LICENSE)

Reading and writing of every `FatturaPA` invoice and SDI message, in Rust.

- the ordinary invoice (`FPR12`, `FPA12`), the simplified invoice (`FSM10`) and the nine SDI messages
- types generated from the official XSD schemas, so a document round-trips through the schema it was written for
- a typed construction API whose `build` runs the official SDI checks
- signed `.xml.p7m` files unwrapped and verified, with the `trust-list` feature reading the `AgID` trust anchors
- an optional `serde` feature

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use std::fs;

use fatturapa::{Document, FatturaElettronica};

let xml = fs::read_to_string("spec/samples/IT01234567890_FPA01.xml")?;
let invoice = FatturaElettronica::from_xml_str(&xml)?;
let numero = &invoice
    .content
    .fattura_elettronica_body[0]
    .content
    .dati_generali
    .content
    .dati_generali_documento
    .content
    .numero;
println!("invoice number: {numero}");
# Ok(())
# }
```

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use fatturapa::ids::{Provincia, RegimeFiscale};
use fatturapa::invoice::{Invoice, InvoiceCurrency, SevenTerLine};
use fatturapa::party::{Address, Business, Seller};
use fatturapa::Document;
use iso_country::Country;
use iso_currency::Currency;

let seller = Seller::try_denomination(
    "ALPHA SRL",
    "02222222222",
    Address::try_italian("VIA ROMA 1", "00100", "ROMA", Provincia::Rm)?,
    RegimeFiscale::Rf01,
)?;
let client = Business::try_foreign(
    Country::US,
    "ACME INC",
    "US123456789",
    Address::try_foreign("FIFTH AVENUE 1", Some("10001"), "NEW YORK", Country::US)?,
)?;
let line = SevenTerLine::builder()
    .description("SERVIZIO DI CONSULENZA")
    .quantity("1.00")
    .unit_price("150.00")
    .build()?;
let doc = Invoice::cross_border_services(&seller, &client, line)
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00012")
    .currency(InvoiceCurrency::try_foreign(Currency::USD, "1.08", "2026-11-04")?)
    .build()?;
let xml = doc.to_xml_string()?;
assert!(xml.contains("versione=\"FPR12\""));
assert!(xml.contains("<CodiceDestinatario>XXXXXXX</CodiceDestinatario>"));
# Ok(())
# }
```
