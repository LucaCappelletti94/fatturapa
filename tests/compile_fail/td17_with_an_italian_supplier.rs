//! The `TD17` self-invoice takes a non-Italian supplier, so an Italian
//! `Seller` is a type error.

fn main() {
    let supplier = fatturapa::party::Seller::try_denomination(
        "ALPHA SRL",
        "02222222222",
        fatturapa::party::Address::try_italian(
            "VIA ROMA 1",
            "00100",
            "ROMA",
            fatturapa::ids::Provincia::Rm,
        )
        .expect("a valid address"),
        fatturapa::ids::RegimeFiscale::Rf01,
    )
    .expect("a valid seller");
    let buyer = fatturapa::party::Company::try_denomination(
        "BETA SRL",
        "04444444444",
        fatturapa::party::Address::try_italian(
            "VIA MILANO 2",
            "20100",
            "MILANO",
            fatturapa::ids::Provincia::Mi,
        )
        .expect("a valid address"),
    )
    .expect("a valid buyer");
    let line = fatturapa::invoice::Line::builder()
        .description("SERVIZIO")
        .quantity("1.00")
        .unit_price("100.00")
        .vat(fatturapa::invoice::VatTreatment::taxed("22.00").expect("a valid rate"))
        .build()
        .expect("the line builds");
    let _ = fatturapa::invoice::SelfInvoice::td17(
        &supplier,
        &buyer,
        fatturapa::invoice::Recipient::try_codice("ABC1234").expect("a valid code"),
        line,
    );
}
