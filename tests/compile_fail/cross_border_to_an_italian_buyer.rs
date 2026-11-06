//! The art. 7-ter invoice takes an EU or non-EU business client, so an
//! Italian `Buyer` is a type error.

fn main() {
    let seller = fatturapa::party::Seller::denomination(
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
    let buyer = fatturapa::party::Buyer::denomination(
        "BETA SRL".parse().expect("a valid denomination"),
        "04444444444".parse().expect("a valid partita IVA"),
        fatturapa::party::Address::italian(
            "VIA MILANO 2".parse().expect("a valid address"),
            "20100".parse().expect("a valid CAP"),
            "MILANO".parse().expect("a valid comune"),
            fatturapa::ids::Provincia::Mi,
        ),
    );
    let line = fatturapa::invoice::SevenTerLine::builder()
        .description("SERVIZIO".parse().expect("a valid description"))
        .unit_price("100.00".parse().expect("a valid decimal"))
        .build();
    let _ = fatturapa::invoice::Invoice::cross_border_services(&seller, &buyer, line);
}
