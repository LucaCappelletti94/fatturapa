//! The `Line` builder requires the VAT treatment, so a line without the
//! `vat` setter does not build.

fn main() {
    let _ = fatturapa::invoice::Line::builder()
        .description("SERVIZIO")
        .quantity("1.00")
        .unit_price("100.00")
        .build();
}
