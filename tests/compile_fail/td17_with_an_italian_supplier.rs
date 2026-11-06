//! The `TD17` self-invoice takes a non-Italian supplier, so an Italian
//! `Seller` is a type error.

fn main() {
    let supplier = fatturapa::party::Seller::denomination(
        "ALPHA SRL".parse().expect("a valid denomination"),
        "02222222222".parse().expect("a valid partita IVA"),
        fatturapa::party::Address::italian(
            "VIA ROMA 1".parse().expect("a valid address"),
            "00100".parse().expect("a valid CAP"),
            "ROMA".parse().expect("a valid comune"),
            fatturapa::ids::Provincia::Rm,
        ),
        fatturapa::ids::RegimeFiscale::Rf01,
    );
    let buyer = fatturapa::party::Company::denomination(
        "BETA SRL".parse().expect("a valid denomination"),
        "04444444444".parse().expect("a valid partita IVA"),
        fatturapa::party::Address::italian(
            "VIA MILANO 2".parse().expect("a valid address"),
            "20100".parse().expect("a valid CAP"),
            "MILANO".parse().expect("a valid comune"),
            fatturapa::ids::Provincia::Mi,
        ),
    );
    let line = fatturapa::invoice::Line::builder()
        .description("SERVIZIO".parse().expect("a valid description"))
        .quantity("1.00".parse().expect("a valid decimal"))
        .unit_price("100.00".parse().expect("a valid decimal"))
        .vat(fatturapa::invoice::VatTreatment::Taxed(
            "22.00".parse().expect("a valid rate"),
        ))
        .build();
    let _ = fatturapa::invoice::SelfInvoice::td17(
        &supplier,
        &buyer,
        fatturapa::invoice::Recipient::Codice(
            "ABC1234".parse().expect("a valid code"),
        ),
        line,
    );
}
