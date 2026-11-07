# fatturapa

[![CI](https://github.com/LucaCappelletti94/fatturapa/actions/workflows/ci.yml/badge.svg)](https://github.com/LucaCappelletti94/fatturapa/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/LucaCappelletti94/fatturapa/graph/badge.svg)](https://codecov.io/gh/LucaCappelletti94/fatturapa)
[![crates.io](https://img.shields.io/crates/v/fatturapa.svg)](https://crates.io/crates/fatturapa)
[![docs.rs](https://docs.rs/fatturapa/badge.svg)](https://docs.rs/fatturapa)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/LucaCappelletti94/fatturapa/blob/main/LICENSE)

Reading and writing of every `FatturaPA` invoice and SDI message, in Rust. Unofficial.

- the ordinary invoice, `FPR12` and `FPA12`
- the simplified invoice, `FSM10`
- the nine SDI messages, from the delivery receipt to the deadline notice
- signed files: a `CAdES` `.xml.p7m` (raw `DER` or `base64`) is unwrapped to its `XML` and its signature verified, with the `trust-list` feature parsing the `AgID` trusted services list into trust anchors
- an optional `serde` feature that reads and writes the typed identifiers and domain types as JSON, validating every value on the way in

The types are generated from the official XSD schemas in `spec/`, so a document round-trips through the schema it was written for. Amounts keep their exact decimal scale and dates keep their lexical form.

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

The construction API is typed. The identifiers validate on construction, the scenario constructors fill what the specification dictates, and the lines of the art. 7-ter invoice carry no VAT treatment, the scenario writing the `N2.1` nature and its reference. Every setter accepts any value convertible into the typed one through `TryFrom`, so the strings above go straight into the API and `build` returns the first conversion error before any other check. A document priced in a foreign currency keeps the `Divisa` `EUR` and every amount in euros, the art. 13 convention, the original amount and the rate carried as management data. `build` runs the official SDI checks of `spec/Elenco-Controlli` before it returns. Signed files read through `fatturapa::signed::unwrap` and `fatturapa::signed::verify`, and the transmission file name builds through `fatturapa::InvoiceName`.
