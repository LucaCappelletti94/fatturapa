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

The types are generated from the official XSD schemas in `spec/`, so a document round-trips through the schema it was written for. Amounts keep their exact decimal scale and dates keep their lexical form.

```rust
use std::fs;

use fatturapa::{Document, FatturaElettronica};

let xml = fs::read_to_string("spec/samples/IT01234567890_FPA01.xml").unwrap();
let invoice = FatturaElettronica::from_xml_str(&xml).unwrap();
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
```

```rust
use fatturapa::ids::{Provincia, RegimeFiscale};
use fatturapa::invoice::{Invoice, InvoiceCurrency, SevenTerLine};
use fatturapa::party::{Address, Business, Seller};
use fatturapa::Document;

let seller = Seller::denomination(
    "ALPHA SRL".parse().unwrap(),
    "02222222222".parse().unwrap(),
    Address::italian(
        "VIA ROMA 1".parse().unwrap(),
        "00100".parse().unwrap(),
        "ROMA".parse().unwrap(),
        Provincia::Rm,
    ),
    RegimeFiscale::Rf01,
);
let client = Business::foreign(
    iso_country::Country::US,
    "ACME INC".parse().unwrap(),
    "US123456789".parse().unwrap(),
    Address::foreign(
        "FIFTH AVENUE 1".parse().unwrap(),
        Some("10001".parse().unwrap()),
        "NEW YORK".parse().unwrap(),
        iso_country::Country::US,
    )
    .unwrap(),
);
let line = SevenTerLine::builder()
    .description("SERVIZIO DI CONSULENZA".parse().unwrap())
    .quantity("1.00".parse().unwrap())
    .unit_price("150.00".parse().unwrap())
    .build();
let doc = Invoice::cross_border_services(&seller, &client, line)
    .progressivo("00001".parse().unwrap())
    .date("2026-11-04".parse().unwrap())
    .numero("00012".parse().unwrap())
    .causali(vec![
        "CONSULENZA AI SENSI DELL'ART. 7-TER DPR 633/72".parse().unwrap(),
    ])
    .currency(InvoiceCurrency::Foreign {
        currency: iso_currency::Currency::USD,
        rate: "1.08".parse().unwrap(),
        rate_date: "2026-11-04".parse().unwrap(),
    })
    .build()
    .unwrap();
let xml = doc.to_xml_string().unwrap();
assert!(xml.contains("versione=\"FPR12\""));
assert!(xml.contains("<CodiceDestinatario>XXXXXXX</CodiceDestinatario>"));
```

The construction API is typed. The identifiers validate on construction, the scenario constructors fill what the specification dictates, and the lines of the art. 7-ter invoice carry no VAT treatment, the scenario writing the `N2.1` nature and its reference. A document priced in a foreign currency keeps the `Divisa` `EUR` and every amount in euros, the art. 13 convention, the original amount and the rate carried as management data. `build` runs the official SDI checks of `spec/Elenco-Controlli` before it returns. Signed files read through `fatturapa::signed::unwrap` and `fatturapa::signed::verify`, and the transmission file name builds through `fatturapa::InvoiceName`.
