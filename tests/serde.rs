//! Round-trip and rejection tests for the `serde` feature of the typed
//! construction layer.
//!
//! Every typed public type serialises to its canonical form and comes back
//! through its validating constructor, so an invalid value is rejected at
//! deserialization time with a meaningful error.
#![cfg(feature = "serde")]
#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test code: each failure names the invariant under test"
)]

use fatturapa::filename::{InvoiceName, MessageKind, MessageName};
use fatturapa::ids::{
    Aliquota, Bic, Cap, CodiceDestinatario, CodiceFiscale, Denomination, Description, EuCountry,
    ExchangeRate, ForeignTaxId, Iban, IdDocumento, Natura, Numero, Pec, PersonName, PostalCode,
    ProgressivoInvio, Provincia, RegimeFiscale, RiferimentoNormativo, Telefono, UnitaMisura,
};
use fatturapa::invoice::{Invoice, InvoiceCurrency, Line, Recipient, Sconto, VatTreatment};
use fatturapa::party::{Address, Business, Buyer, Company, Seller, TaxIdentity};
use fatturapa::{Amount, Date, DateTime};
use iso_country::Country as IsoCountry;
use iso_currency::Currency;

/// Serialise `value` to JSON and back, then compare the round trip to the input.
fn roundtrip<T>(value: &T) -> T
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + PartialEq,
{
    let json = serde_json::to_string(value).expect("serialises to JSON");
    let back: T =
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("round-trips: {json} ({e})"));
    assert_eq!(value, &back);
    back
}

#[test]
fn identifier_newtypes_roundtrip() {
    let partita: fatturapa::ids::PartitaIva = "02222222222".parse().unwrap();
    assert_eq!(roundtrip(&partita), partita);
    assert_eq!(
        roundtrip(&"04444444444".parse::<fatturapa::ids::PartitaIva>().unwrap()),
        "04444444444".parse().unwrap()
    );

    let codice_fiscale: CodiceFiscale = "MRALNE80E05H501C".parse().unwrap();
    assert_eq!(roundtrip(&codice_fiscale), codice_fiscale);

    assert_eq!(
        roundtrip(&"00100".parse::<Cap>().unwrap()),
        "00100".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"SEPA123".parse::<CodiceDestinatario>().unwrap()),
        "SEPA123".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"00001".parse::<ProgressivoInvio>().unwrap()),
        "00001".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"DEUTDEFF501".parse::<Bic>().unwrap()),
        "DEUTDEFF501".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"US123456789".parse::<ForeignTaxId>().unwrap()),
        "US123456789".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"0211234567".parse::<Telefono>().unwrap()),
        "0211234567".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"IT60X0542811101000000123456".parse::<Iban>().unwrap()),
        "IT60X0542811101000000123456".parse().unwrap()
    );
    assert_eq!(
        roundtrip(
            &"ART. 7-TER DPR 633/72"
                .parse::<RiferimentoNormativo>()
                .unwrap()
        ),
        "ART. 7-TER DPR 633/72".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"1".parse::<Numero>().unwrap()),
        "1".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"ORIG123".parse::<IdDocumento>().unwrap()),
        "ORIG123".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"causale text".parse::<fatturapa::ids::Causale>().unwrap()),
        "causale text".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"a line item".parse::<Description>().unwrap()),
        "a line item".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"ACME SPA".parse::<Denomination>().unwrap()),
        "ACME SPA".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"Mario".parse::<PersonName>().unwrap()),
        "Mario".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"VIA ROMA 1".parse::<fatturapa::ids::Indirizzo>().unwrap()),
        "VIA ROMA 1".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"ROMA".parse::<fatturapa::ids::Comune>().unwrap()),
        "ROMA".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"LA".parse::<UnitaMisura>().unwrap()),
        "LA".parse().unwrap()
    );
    assert_eq!(
        roundtrip(&"10001".parse::<PostalCode>().unwrap()),
        "10001".parse().unwrap()
    );

    let pec: Pec = "mario.rossi@pec.it".parse().unwrap();
    assert_eq!(roundtrip(&pec), pec);
    let email: fatturapa::ids::Email = "mario.rossi@example.com".parse().unwrap();
    assert_eq!(roundtrip(&email), email);
}

#[test]
fn decimal_types_keep_scale() {
    let amount: Amount = "100.00".parse().unwrap();
    assert_eq!(serde_json::to_string(&amount).unwrap(), "\"100.00\"");
    assert_eq!(roundtrip(&amount), amount);

    let aliquota: Aliquota = "22.00".parse().unwrap();
    assert_eq!(serde_json::to_string(&aliquota).unwrap(), "\"22.00\"");
    assert_eq!(roundtrip(&aliquota), aliquota);

    let rate: ExchangeRate = "1.0800".parse().unwrap();
    assert_eq!(serde_json::to_string(&rate).unwrap(), "\"1.0800\"");
    assert_eq!(roundtrip(&rate), rate);
}

#[test]
fn date_types_use_lexical_form() {
    let date: Date = "2024-01-15".parse().unwrap();
    assert_eq!(serde_json::to_string(&date).unwrap(), "\"2024-01-15\"");
    assert_eq!(roundtrip(&date), date);

    let datetime: DateTime = "2024-01-15T10:30:00Z".parse().unwrap();
    assert_eq!(roundtrip(&datetime), datetime);
}

#[test]
fn code_enums_serialise_as_official_codes() {
    assert_eq!(serde_json::to_string(&Provincia::Rm).unwrap(), "\"RM\"");
    assert_eq!(serde_json::to_string(&EuCountry::De).unwrap(), "\"DE\"");
    assert_eq!(serde_json::to_string(&Natura::N2_1).unwrap(), "\"N2.1\"");
    assert_eq!(
        serde_json::to_string(&RegimeFiscale::Rf01).unwrap(),
        "\"RF01\""
    );

    assert_eq!(roundtrip(&Provincia::Rm), Provincia::Rm);
    assert_eq!(roundtrip(&EuCountry::De), EuCountry::De);
    assert_eq!(roundtrip(&Natura::N2_1), Natura::N2_1);
    assert_eq!(roundtrip(&RegimeFiscale::Rf01), RegimeFiscale::Rf01);

    let provincia: Provincia = serde_json::from_str("\"MI\"").unwrap();
    assert_eq!(provincia, Provincia::Mi);
}

#[test]
fn message_kind_and_names_serialise_as_their_form() {
    assert_eq!(serde_json::to_string(&MessageKind::At).unwrap(), "\"AT\"");
    assert_eq!(roundtrip(&MessageKind::At), MessageKind::At);

    let invoice_name = InvoiceName::new("IT", "0123456789A", "00001").unwrap();
    assert_eq!(
        serde_json::to_string(&invoice_name).unwrap(),
        format!("\"{invoice_name}\"")
    );
    assert_eq!(roundtrip(&invoice_name), invoice_name);

    let message_name = MessageName::new(invoice_name, MessageKind::At, "001").unwrap();
    assert_eq!(
        serde_json::to_string(&message_name).unwrap(),
        format!("\"{message_name}\"")
    );
    assert_eq!(roundtrip(&message_name), message_name);
}

#[test]
fn domain_types_roundtrip() {
    let tax_identity =
        TaxIdentity::try_italian(Some("02222222222"), Some("MRALNE80E05H501C")).unwrap();
    assert_eq!(roundtrip(&tax_identity), tax_identity);

    let address = Address::italian(
        "VIA ROMA 1".parse().unwrap(),
        "00100".parse().unwrap(),
        "ROMA".parse().unwrap(),
        Provincia::Rm,
    );
    assert_eq!(roundtrip(&address), address);

    let foreign_address = Address::foreign(
        "1 MAIN ST".parse().unwrap(),
        Some("10001".parse().unwrap()),
        "NEW YORK".parse().unwrap(),
        IsoCountry::US,
    )
    .unwrap();
    assert_eq!(roundtrip(&foreign_address), foreign_address);

    let seller = Seller::denomination(
        "ACME SPA".parse().unwrap(),
        "02222222222".parse().unwrap(),
        Address::italian(
            "VIA ROMA 1".parse().unwrap(),
            "00100".parse().unwrap(),
            "ROMA".parse().unwrap(),
            Provincia::Rm,
        ),
        RegimeFiscale::Rf01,
    );
    assert_eq!(roundtrip(&seller), seller);

    // A `Party` round-trips on its own, reached through the `seller`.
    let party = seller.party().clone();
    assert_eq!(roundtrip(&party), party);

    let buyer = Buyer::denomination(
        "BETA SPA".parse().unwrap(),
        "04444444444".parse().unwrap(),
        Address::italian(
            "VIA MILANO 2".parse().unwrap(),
            "20100".parse().unwrap(),
            "MILANO".parse().unwrap(),
            Provincia::Mi,
        ),
    );
    assert_eq!(roundtrip(&buyer), buyer);

    let business = Business::foreign(
        IsoCountry::US,
        "ACME SPA".parse().unwrap(),
        "US123456789".parse().unwrap(),
        foreign_address,
    );
    assert_eq!(roundtrip(&business), business);

    let company = Company::denomination(
        "BETA SPA".parse().unwrap(),
        "04444444444".parse().unwrap(),
        Address::italian(
            "VIA MILANO 2".parse().unwrap(),
            "20100".parse().unwrap(),
            "MILANO".parse().unwrap(),
            Provincia::Mi,
        ),
    );
    assert_eq!(roundtrip(&company), company);

    let vat = VatTreatment::taxed("22.00").unwrap();
    assert_eq!(roundtrip(&vat), vat);

    let not_taxed = VatTreatment::NotTaxed {
        natura: Natura::N2_1,
        riferimento_normativo: "ART. 7-TER DPR 633/72".parse().unwrap(),
    };
    assert_eq!(roundtrip(&not_taxed), not_taxed);

    let line = Line::builder()
        .description("a line item")
        .unit_price("100.00")
        .vat(vat)
        .build()
        .unwrap();
    assert_eq!(roundtrip(&line), line);

    let recipient = Recipient::try_codice("SEPA123").unwrap();
    assert_eq!(roundtrip(&recipient), recipient);

    let euro = InvoiceCurrency::Eur;
    assert_eq!(roundtrip(&euro), euro);

    let foreign = InvoiceCurrency::try_foreign(Currency::USD, "1.0800", "2024-01-15").unwrap();
    assert_eq!(roundtrip(&foreign), foreign);

    let sconto = Sconto::discount_percent("10.00").unwrap();
    assert_eq!(roundtrip(&sconto), sconto);
}

#[test]
fn deserialization_rejects_invalid_values() {
    // A `partita IVA` with a wrong check digit is rejected.
    let err = serde_json::from_str::<fatturapa::ids::PartitaIva>("\"02222222223\"").unwrap_err();
    assert!(err.to_string().contains("partita"), "meaningful: {err}");

    // An `aliquota` above `100` is rejected.
    let err = serde_json::from_str::<Aliquota>("\"101.00\"").unwrap_err();
    assert!(err.to_string().contains("rate"), "meaningful: {err}");

    // An `exchange rate` with five decimals is rejected.
    let err = serde_json::from_str::<ExchangeRate>("\"1.08001\"").unwrap_err();
    assert!(err.to_string().contains("rate"), "meaningful: {err}");

    // A `date` in a non-lexical form is rejected.
    let err = serde_json::from_str::<Date>("\"not-a-date\"").unwrap_err();
    assert!(err.to_string().contains("date"), "meaningful: {err}");

    // A `seller` whose `partita IVA` is absent is rejected.
    let err = serde_json::from_str::<Seller>(
        r#"{"party":{"name":{"Denomination":"ACME SPA"},
            "identity":{"Italian":{"partita_iva":null,"codice_fiscale":"MRALNE80E05H501C"}},
            "address":{"Italian":{"indirizzo":"VIA ROMA 1","cap":"00100","comune":"ROMA","provincia":"RM"}}},
            "regime":"RF01"}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("seller"), "meaningful: {err}");

    // A `business` whose party is `Italian` is rejected.
    let err = serde_json::from_str::<Business>(
        r#"{"party":{"name":{"Denomination":"ACME SPA"},
            "identity":{"Italian":{"partita_iva":"02222222222","codice_fiscale":null}},
            "address":{"Italian":{"indirizzo":"VIA ROMA 1","cap":"00100","comune":"ROMA","provincia":"RM"}}}}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("business"), "meaningful: {err}");

    // A `company` whose party has no `partita IVA` is rejected.
    let err = serde_json::from_str::<Company>(
        r#"{"party":{"name":{"Denomination":"BETA SPA"},
            "identity":{"Italian":{"partita_iva":null,"codice_fiscale":"MRALNE80E05H501C"}},
            "address":{"Italian":{"indirizzo":"VIA MILANO 2","cap":"20100","comune":"MILANO","provincia":"MI"}}}}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("company"), "meaningful: {err}");

    // A `positive rate on a not-taxed nature` is rejected because `Taxed`
    // requires a positive `aliquota` and a zero rate fails that invariant.
    let err = serde_json::from_str::<VatTreatment>(r#"{"Taxed":"0.00"}"#).unwrap_err();
    assert!(err.to_string().contains("rate"), "meaningful: {err}");

    // A `not-taxed` block with an unknown `natura` code is rejected.
    let err = serde_json::from_str::<VatTreatment>(
        r#"{"NotTaxed":{"natura":"ZZ","riferimento_normativo":"X"}}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("natura"), "meaningful: {err}");

    // A `foreign` address exceeding the `Indirizzo` length is rejected.
    let too_long = "A".repeat(80);
    let err = serde_json::from_str::<Address>(&format!(
        r#"{{"Foreign":{{"indirizzo":{too_long:?},"postal_code":"10001","comune":"NEW YORK","country":"US"}}}}"#
    ))
    .unwrap_err();
    assert!(err.to_string().contains("address"), "meaningful: {err}");

    // A `tax identity` for `Italy` with no identifier is rejected.
    let err = serde_json::from_str::<TaxIdentity>(
        r#"{"Italian":{"partita_iva":null,"codice_fiscale":null}}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("identity"), "meaningful: {err}");
}

#[test]
fn document_from_deserialized_parties_equals_typed() {
    let seller = Seller::denomination(
        "ACME SPA".parse().unwrap(),
        "02222222222".parse().unwrap(),
        Address::italian(
            "VIA ROMA 1".parse().unwrap(),
            "00100".parse().unwrap(),
            "ROMA".parse().unwrap(),
            Provincia::Rm,
        ),
        RegimeFiscale::Rf01,
    );
    let buyer = Buyer::denomination(
        "BETA SPA".parse().unwrap(),
        "04444444444".parse().unwrap(),
        Address::italian(
            "VIA MILANO 2".parse().unwrap(),
            "20100".parse().unwrap(),
            "MILANO".parse().unwrap(),
            Provincia::Mi,
        ),
    );
    let line = Line::builder()
        .description("a line item")
        .unit_price("100.00")
        .vat(VatTreatment::taxed("22.00").unwrap())
        .build()
        .unwrap();

    // Round-trip the parties and the line through JSON.
    let seller_json = serde_json::to_string(&seller).unwrap();
    let buyer_json = serde_json::to_string(&buyer).unwrap();
    let line_json = serde_json::to_string(&line).unwrap();

    let seller_back: Seller = serde_json::from_str(&seller_json).unwrap();
    let buyer_back: Buyer = serde_json::from_str(&buyer_json).unwrap();
    let line_back: Line = serde_json::from_str(&line_json).unwrap();

    assert_eq!(seller, seller_back);
    assert_eq!(buyer, buyer_back);
    assert_eq!(line, line_back);

    let recipient = Recipient::try_codice("SEPA123").unwrap();
    let doc_typed = Invoice::domestic(&seller, &buyer, recipient.clone(), line)
        .progressivo("00001")
        .date("2024-01-15")
        .numero("1")
        .build()
        .unwrap();
    let doc_back = Invoice::domestic(&seller_back, &buyer_back, recipient, line_back)
        .progressivo("00001")
        .date("2024-01-15")
        .numero("1")
        .build()
        .unwrap();

    assert_eq!(doc_typed, doc_back);
}
