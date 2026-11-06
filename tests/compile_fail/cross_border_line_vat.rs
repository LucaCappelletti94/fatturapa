//! The art. 7-ter line carries no VAT treatment of its own, the scenario
//! writing the `N2.1` nature, so the `vat` setter does not exist on its
//! builder.

fn main() {
    let _ = fatturapa::invoice::SevenTerLine::builder()
        .description("SERVIZIO".parse().expect("a valid description"))
        .unit_price("100.00".parse().expect("a valid decimal"))
        .vat(fatturapa::invoice::VatTreatment::Taxed(
            "22.00".parse().expect("a valid rate"),
        ))
        .build();
}
