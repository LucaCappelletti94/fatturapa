//! The `Line` builder requires the VAT treatment, so a line without the
//! `vat` setter does not build.

fn main() {
    let _ = fatturapa::invoice::Line::builder()
        .description("SERVIZIO".parse().expect("a valid description"))
        .quantity("1.00".parse().expect("a valid decimal"))
        .unit_price("100.00".parse().expect("a valid decimal"))
        .build();
}
