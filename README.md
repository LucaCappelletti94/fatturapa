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
use fatturapa::builder::{Invoice, InvoiceBody, Line};
use fatturapa::generated::fpr12 as g;
use fatturapa::Document;

let seller = g::CedentePrestatoreType::new(
    g::IdFiscaleType::new("IT", "01234567890"),
    g::AnagraficaType::denominazione("ALPHA SRL"),
    g::RegimeFiscaleType::Rf01,
    g::IndirizzoType::new("VIA ROMA 1", "00100", "ROMA", "IT"),
);
let buyer = g::CessionarioCommittenteType::new(
    g::IdFiscaleType::new("IT", "09876543210"),
    g::AnagraficaType::denominazione("BETA SRL"),
    g::IndirizzoType::new("VIA MILANO 2", "20100", "MILANO", "IT"),
);
let doc = Invoice::new(g::FormatoTrasmissioneType::Fpr12, "00001", seller, buyer)
    .codice_destinatario("ABC1234")
    .body(
        InvoiceBody::new(
            g::TipoDocumentoType::Td01,
            "EUR",
            "2026-11-04".parse().unwrap(),
            "00013",
        )
        .line(
            Line::new(1, "MATERIALE UFFICIO", "10.00".parse().unwrap(), "22.00".parse().unwrap())
                .quantita("3.00".parse().unwrap()),
        ),
    )
    .build()
    .unwrap();
let xml = doc.to_xml_string().unwrap();
assert!(xml.contains("versione=\"FPR12\""));
```

`build` runs the official SDI checks of `spec/Elenco-Controlli` before it returns. Signed files read through `fatturapa::signed::unwrap` and `fatturapa::signed::verify`, and the transmission file name builds through `fatturapa::InvoiceName`.
