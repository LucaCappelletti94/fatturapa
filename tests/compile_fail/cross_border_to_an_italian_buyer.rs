//! The art. 7-ter invoice takes an EU or non-EU business client, so an
//! Italian `Buyer` is a type error.

fn main() {
    let seller = fatturapa::party::Seller::try_denomination(
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
    let buyer = fatturapa::party::Buyer::try_denomination(
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
    let line = fatturapa::invoice::SevenTerLine::builder()
        .description("SERVIZIO")
        .unit_price("100.00")
        .build()
        .expect("the line builds");
    let _ = fatturapa::invoice::Invoice::cross_border_services(&seller, &buyer, line);
}
