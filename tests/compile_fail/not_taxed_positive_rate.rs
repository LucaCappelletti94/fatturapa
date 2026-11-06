//! The `NotTaxed` form carries no rate, so a positive rate on a non-taxed
//! nature cannot be written.

fn main() {
    let _ = fatturapa::invoice::VatTreatment::NotTaxed {
        natura: fatturapa::ids::Natura::N2_1,
        riferimento_normativo: "ART. 7-TER DPR 633/72"
            .parse()
            .expect("a valid reference"),
        aliquota: fatturapa::ids::Aliquota::try_new(22).expect("a valid rate"),
    };
}
