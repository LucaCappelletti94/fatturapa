//! The document generators shared by the round-trip property tests and
//! the fuzz round-trip target. A `u64` seed builds an arbitrary document
//! of one root type, every value in the pools respecting the facets of the
//! schemas, so the written documents validate and the round trip is a
//! statement about the serialisation and the parser.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

use fatturapa::amount::Amount;
use fatturapa::date::{Date, DateTime};
use fatturapa::generated::{fpr12, fsm10, sdi};
use fatturapa::{
    AttestazioneTrasmissioneFattura, FatturaElettronica, FatturaElettronicaSemplificata,
    MetadatiInvioFile, NotificaDecorrenzaTermini, NotificaEsito, NotificaEsitoCommittente,
    NotificaMancataConsegna, NotificaScarto, RicevutaConsegna, ScartoEsitoCommittente,
};
use xsd_parser_types::xml::Base64String;

/// A deterministic single-thread RNG, seeded per test case.
pub(crate) struct Rng(u64);

impl Rng {
    /// Builds the RNG from a seed.
    #[must_use]
    const fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    /// The next word of the xorshift sequence.
    const fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A value in `0..n`.
    const fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// A random element of `items`.
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let n = u64::try_from(items.len()).expect("the pool is small");
        &items[usize::try_from(self.below(n)).expect("fits an index")]
    }

    /// A random boolean.
    const fn bool(&mut self) -> bool {
        (self.next() & 1) == 0
    }
}

const COUNTRIES: [&str; 3] = ["IT", "FR", "DE"];
const CITIES: [&str; 3] = ["Milano", "Roma", "Torino"];
const STREETS: [&str; 3] = ["Via Roma", "Corso Vittorio", "Viale Europa"];
const DENOMS: [&str; 3] = ["Societa Alfa", "Studio Beta", "Ditta Gamma"];
const FIRST: [&str; 3] = ["Mario", "Anna", "Luca"];
const LAST: [&str; 3] = ["Rossi", "Bianchi", "Conti"];
const DATES: [&str; 4] = ["2014-11-20", "2016-05-15", "2024-01-02", "2026-08-30"];
const DATETIMES: [&str; 4] = [
    "2014-11-20T10:30:00Z",
    "2014-11-20T10:30:00+02:00",
    "2014-11-20T10:30:00.500Z",
    "2014-11-20T10:30:00",
];
const AMOUNTS: [&str; 4] = ["123.45", "2.00", "10.00", "100.00"];
const RATES: [&str; 3] = ["22.00", "10.00", "4.00"];

pub(crate) fn amount(rng: &mut Rng) -> Amount {
    rng.pick(&AMOUNTS).parse().expect("the pool is valid")
}

/// A VAT rate, within the schema `maxInclusive 100.00`.
pub(crate) fn rate(rng: &mut Rng) -> Amount {
    rng.pick(&RATES).parse().expect("the pool is valid")
}

pub(crate) fn date(rng: &mut Rng) -> Date {
    rng.pick(&DATES).parse().expect("the pool is valid")
}

pub(crate) fn datetime(rng: &mut Rng) -> DateTime {
    rng.pick(&DATETIMES).parse().expect("the pool is valid")
}

pub(crate) fn codice_destinatario(rng: &mut Rng) -> String {
    const ALPHABET: [u8; 36] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut chars = [0_u8; 7];
    for char in &mut chars {
        *char = ALPHABET[usize::try_from(rng.below(36)).expect("fits an index")];
    }
    String::from_utf8(chars.to_vec()).expect("the pool is ASCII")
}

/// An ordinary invoice with a random `FPA12`/`FPR12` header and a single
/// line and a single VAT summary.
#[expect(
    clippy::too_many_lines,
    reason = "the generator builds the whole document in one place"
)]
pub(crate) fn invoice_from_seed(seed: u64) -> FatturaElettronica {
    let mut rng = Rng::new(seed);
    let versione = rng
        .pick(&[
            fpr12::FormatoTrasmissioneType::Fpa12,
            fpr12::FormatoTrasmissioneType::Fpr12,
        ])
        .clone();
    FatturaElettronica {
        versione,
        sistema_emittente: if rng.bool() {
            Some("Sistema".to_string())
        } else {
            None
        },
        content: fpr12::FatturaElettronicaTypeContent {
            fattura_elettronica_header: fpr12::FatturaElettronicaHeaderType {
                content: fpr12::FatturaElettronicaHeaderTypeContent {
                    dati_trasmissione: fpr12::DatiTrasmissioneType {
                        content: fpr12::DatiTrasmissioneTypeContent {
                            id_trasmittente: fpr12::IdFiscaleType {
                                content: fpr12::IdFiscaleTypeContent {
                                    id_paese: "IT".to_string(),
                                    id_codice: format!("{:011}", rng.below(100_000_000_000)),
                                },
                            },
                            progressivo_invio: format!("{:0>4}", rng.below(10_000)),
                            formato_trasmissione: rng
                                .pick(&[
                                    fpr12::FormatoTrasmissioneType::Fpa12,
                                    fpr12::FormatoTrasmissioneType::Fpr12,
                                ])
                                .clone(),
                            codice_destinatario: codice_destinatario(&mut rng),
                            contatti_trasmittente: None,
                            pec_destinatario: if rng.bool() {
                                Some("prova@agenzia.it".to_string())
                            } else {
                                None
                            },
                        },
                    },
                    cedente_prestatore: fpr12::CedentePrestatoreType {
                        content: fpr12::CedentePrestatoreTypeContent {
                            dati_anagrafici: fpr12::DatiAnagraficiCedenteType {
                                content: fpr12::DatiAnagraficiCedenteTypeContent {
                                    id_fiscale_iva: fpr12::IdFiscaleType {
                                        content: fpr12::IdFiscaleTypeContent {
                                            id_paese: "IT".to_string(),
                                            id_codice: format!(
                                                "{:011}",
                                                rng.below(100_000_000_000)
                                            ),
                                        },
                                    },
                                    codice_fiscale: None,
                                    anagrafica: fpr12::AnagraficaType {
                                        content: fpr12::AnagraficaTypeContent {
                                            content_60: fpr12::AnagraficaContent60Type {
                                                content: if rng.bool() {
                                                    fpr12::AnagraficaContent60TypeContent::Content61(
                                                        fpr12::AnagraficaContent61Type {
                                                            content: fpr12::AnagraficaContent61TypeContent {
                                                                denominazione: rng
                                                                    .pick(&DENOMS)
                                                                    .to_string(),
                                                            },
                                                        },
                                                    )
                                                } else {
                                                    fpr12::AnagraficaContent60TypeContent::Content62(
                                                        fpr12::AnagraficaContent62Type {
                                                            content: fpr12::AnagraficaContent62TypeContent {
                                                                nome: rng.pick(&FIRST).to_string(),
                                                                cognome: rng.pick(&LAST).to_string(),
                                                            },
                                                        },
                                                    )
                                                },
                                            },
                                            titolo: None,
                                            cod_eori: None,
                                        },
                                    },
                                    albo_professionale: None,
                                    provincia_albo: None,
                                    numero_iscrizione_albo: None,
                                    data_iscrizione_albo: None,
                                    regime_fiscale: rng
                                        .pick(&[
                                            fpr12::RegimeFiscaleType::Rf01,
                                            fpr12::RegimeFiscaleType::Rf04,
                                            fpr12::RegimeFiscaleType::Rf05,
                                        ])
                                        .clone(),
                                },
                            },
                            sede: fpr12::IndirizzoType {
                                content: fpr12::IndirizzoTypeContent {
                                    indirizzo: rng.pick(&STREETS).to_string(),
                                    numero_civico: None,
                                    cap: format!("{:05}", rng.below(100_000)),
                                    comune: rng.pick(&CITIES).to_string(),
                                    provincia: None,
                                    nazione: rng.pick(&COUNTRIES).to_string(),
                                },
                            },
                            stabile_organizzazione: None,
                            iscrizione_rea: None,
                            contatti: None,
                            riferimento_amministrazione: None,
                        },
                    },
                    rappresentante_fiscale: None,
                    cessionario_committente: fpr12::CessionarioCommittenteType {
                        content: fpr12::CessionarioCommittenteTypeContent {
                            dati_anagrafici: fpr12::DatiAnagraficiCessionarioType {
                                content: fpr12::DatiAnagraficiCessionarioTypeContent {
                                    id_fiscale_iva: Some(fpr12::IdFiscaleType {
                                        content: fpr12::IdFiscaleTypeContent {
                                            id_paese: "IT".to_string(),
                                            id_codice: format!(
                                                "{:011}",
                                                rng.below(100_000_000_000)
                                            ),
                                        },
                                    }),
                                    codice_fiscale: None,
                                    anagrafica: fpr12::AnagraficaType {
                                        content: fpr12::AnagraficaTypeContent {
                                            content_60: fpr12::AnagraficaContent60Type {
                                                content: if rng.bool() {
                                                    fpr12::AnagraficaContent60TypeContent::Content61(
                                                        fpr12::AnagraficaContent61Type {
                                                            content: fpr12::AnagraficaContent61TypeContent {
                                                                denominazione: rng
                                                                    .pick(&DENOMS)
                                                                    .to_string(),
                                                            },
                                                        },
                                                    )
                                                } else {
                                                    fpr12::AnagraficaContent60TypeContent::Content62(
                                                        fpr12::AnagraficaContent62Type {
                                                            content: fpr12::AnagraficaContent62TypeContent {
                                                                nome: rng.pick(&FIRST).to_string(),
                                                                cognome: rng.pick(&LAST).to_string(),
                                                            },
                                                        },
                                                    )
                                                },
                                            },
                                            titolo: None,
                                            cod_eori: None,
                                        },
                                    },
                                },
                            },
                            sede: fpr12::IndirizzoType {
                                content: fpr12::IndirizzoTypeContent {
                                    indirizzo: rng.pick(&STREETS).to_string(),
                                    numero_civico: None,
                                    cap: format!("{:05}", rng.below(100_000)),
                                    comune: rng.pick(&CITIES).to_string(),
                                    provincia: None,
                                    nazione: rng.pick(&COUNTRIES).to_string(),
                                },
                            },
                            stabile_organizzazione: None,
                            rappresentante_fiscale: None,
                        },
                    },
                    terzo_intermediario_o_soggetto_emittente: None,
                    soggetto_emittente: if rng.bool() {
                        Some(
                            rng.pick(&[
                                fpr12::SoggettoEmittenteType::Cc,
                                fpr12::SoggettoEmittenteType::Tz,
                            ])
                            .clone(),
                        )
                    } else {
                        None
                    },
                },
            },
            fattura_elettronica_body: vec![fpr12::FatturaElettronicaBodyType {
                content: fpr12::FatturaElettronicaBodyTypeContent {
                    dati_generali: fpr12::DatiGeneraliType {
                        content: fpr12::DatiGeneraliTypeContent {
                            dati_generali_documento: fpr12::DatiGeneraliDocumentoType {
                                content: fpr12::DatiGeneraliDocumentoTypeContent {
                                    tipo_documento: rng
                                        .pick(&[
                                            fpr12::TipoDocumentoType::Td01,
                                            fpr12::TipoDocumentoType::Td02,
                                            fpr12::TipoDocumentoType::Td03,
                                            fpr12::TipoDocumentoType::Td04,
                                            fpr12::TipoDocumentoType::Td05,
                                        ])
                                        .clone(),
                                    divisa: "EUR".to_string(),
                                    data: date(&mut rng),
                                    numero: format!("{:0>6}", rng.below(1_000_000)),
                                    dati_ritenuta: vec![],
                                    dati_bollo: None,
                                    dati_cassa_previdenziale: vec![],
                                    sconto_maggiorazione: vec![],
                                    importo_totale_documento: Some(amount(&mut rng)),
                                    arrotondamento: None,
                                    causale: vec![],
                                    art_73: None,
                                },
                            },
                            dati_ordine_acquisto: vec![],
                            dati_contratto: vec![],
                            dati_convenzione: vec![],
                            dati_ricezione: vec![],
                            dati_fatture_collegate: vec![],
                            dati_sal: vec![],
                            dati_ddt: vec![],
                            dati_trasporto: None,
                            fattura_principale: None,
                        },
                    },
                    dati_beni_servizi: fpr12::DatiBeniServiziType {
                        content: fpr12::DatiBeniServiziTypeContent {
                            dettaglio_linee: vec![fpr12::DettaglioLineeType {
                                content: fpr12::DettaglioLineeTypeContent {
                                    numero_linea: 1,
                                    tipo_cessione_prestazione: None,
                                    codice_articolo: vec![],
                                    descrizione: "Servizio di consulenza".to_string(),
                                    quantita: Some(amount(&mut rng)),
                                    unita_misura: None,
                                    data_inizio_periodo: None,
                                    data_fine_periodo: None,
                                    prezzo_unitario: amount(&mut rng),
                                    sconto_maggiorazione: vec![],
                                    prezzo_totale: amount(&mut rng),
                                    aliquota_iva: rate(&mut rng),
                                    ritenuta: None,
                                    natura: if rng.bool() {
                                        Some(
                                            rng.pick(&[
                                                fpr12::NaturaType::N1,
                                                fpr12::NaturaType::N2,
                                            ])
                                            .clone(),
                                        )
                                    } else {
                                        None
                                    },
                                    riferimento_amministrazione: None,
                                    altri_dati_gestionali: vec![],
                                },
                            }],
                            dati_riepilogo: vec![fpr12::DatiRiepilogoType {
                                content: fpr12::DatiRiepilogoTypeContent {
                                    aliquota_iva: rate(&mut rng),
                                    natura: None,
                                    spese_accessorie: None,
                                    arrotondamento: None,
                                    imponibile_importo: amount(&mut rng),
                                    imposta: amount(&mut rng),
                                    esigibilita_iva: if rng.bool() {
                                        Some(
                                            rng.pick(&[
                                                fpr12::EsigibilitaIvaType::D,
                                                fpr12::EsigibilitaIvaType::I,
                                                fpr12::EsigibilitaIvaType::S,
                                            ])
                                            .clone(),
                                        )
                                    } else {
                                        None
                                    },
                                    riferimento_normativo: None,
                                },
                            }],
                        },
                    },
                    dati_veicoli: None,
                    dati_pagamento: vec![],
                    allegati: vec![],
                },
            }],
            signature: None,
        },
    }
}

/// A simplified invoice with a random cedente anagrafica flavour and a
/// single body.
#[expect(
    clippy::too_many_lines,
    reason = "the generator builds the whole document in one place"
)]
pub(crate) fn semplificata_from_seed(seed: u64) -> FatturaElettronicaSemplificata {
    let mut rng = Rng::new(seed);
    FatturaElettronicaSemplificata {
        versione: fsm10::FormatoTrasmissioneType::Fsm10,
        sistema_emittente: if rng.bool() {
            Some("Sistema".to_string())
        } else {
            None
        },
        content: fsm10::FatturaElettronicaTypeContent {
            fattura_elettronica_header: fsm10::FatturaElettronicaHeaderType {
                content: fsm10::FatturaElettronicaHeaderTypeContent {
                    dati_trasmissione: fsm10::DatiTrasmissioneType {
                        content: fsm10::DatiTrasmissioneTypeContent {
                            id_trasmittente: fsm10::IdFiscaleType {
                                content: fsm10::IdFiscaleTypeContent {
                                    id_paese: "IT".to_string(),
                                    id_codice: format!("{:011}", rng.below(100_000_000_000)),
                                },
                            },
                            progressivo_invio: format!("{:0>4}", rng.below(10_000)),
                            formato_trasmissione: fsm10::FormatoTrasmissioneType::Fsm10,
                            codice_destinatario: codice_destinatario(&mut rng),
                            pec_destinatario: if rng.bool() {
                                Some("prova@agenzia.it".to_string())
                            } else {
                                None
                            },
                        },
                    },
                    cedente_prestatore: fsm10::CedentePrestatoreType {
                        content: fsm10::CedentePrestatoreTypeContent {
                            id_fiscale_iva: fsm10::IdFiscaleType {
                                content: fsm10::IdFiscaleTypeContent {
                                    id_paese: "IT".to_string(),
                                    id_codice: format!("{:011}", rng.below(100_000_000_000)),
                                },
                            },
                            codice_fiscale: None,
                            content_49: fsm10::CedentePrestatoreContent49Type {
                                content: if rng.bool() {
                                    fsm10::CedentePrestatoreContent49TypeContent::Content50(
                                        fsm10::CedentePrestatoreContent50Type {
                                            content: fsm10::CedentePrestatoreContent50TypeContent {
                                                denominazione: rng.pick(&DENOMS).to_string(),
                                            },
                                        },
                                    )
                                } else {
                                    fsm10::CedentePrestatoreContent49TypeContent::Content51(
                                        fsm10::CedentePrestatoreContent51Type {
                                            content: fsm10::CedentePrestatoreContent51TypeContent {
                                                nome: rng.pick(&FIRST).to_string(),
                                                cognome: rng.pick(&LAST).to_string(),
                                            },
                                        },
                                    )
                                },
                            },
                            sede: fsm10::IndirizzoType {
                                content: fsm10::IndirizzoTypeContent {
                                    indirizzo: rng.pick(&STREETS).to_string(),
                                    numero_civico: None,
                                    cap: format!("{:05}", rng.below(100_000)),
                                    comune: rng.pick(&CITIES).to_string(),
                                    provincia: None,
                                    nazione: rng.pick(&COUNTRIES).to_string(),
                                },
                            },
                            stabile_organizzazione: None,
                            rappresentante_fiscale: None,
                            iscrizione_rea: None,
                            regime_fiscale: rng
                                .pick(&[
                                    fsm10::RegimeFiscaleType::Rf01,
                                    fsm10::RegimeFiscaleType::Rf04,
                                    fsm10::RegimeFiscaleType::Rf05,
                                ])
                                .clone(),
                        },
                    },
                    cessionario_committente: fsm10::CessionarioCommittenteType {
                        content: fsm10::CessionarioCommittenteTypeContent {
                            identificativi_fiscali: fsm10::IdentificativiFiscaliType {
                                content: fsm10::IdentificativiFiscaliTypeContent {
                                    id_fiscale_iva: Some(fsm10::IdFiscaleType {
                                        content: fsm10::IdFiscaleTypeContent {
                                            id_paese: "IT".to_string(),
                                            id_codice: format!(
                                                "{:011}",
                                                rng.below(100_000_000_000)
                                            ),
                                        },
                                    }),
                                    codice_fiscale: None,
                                },
                            },
                            altri_dati_identificativi: None,
                        },
                    },
                    soggetto_emittente: if rng.bool() {
                        Some(
                            rng.pick(&[
                                fsm10::SoggettoEmittenteType::Cc,
                                fsm10::SoggettoEmittenteType::Tz,
                            ])
                            .clone(),
                        )
                    } else {
                        None
                    },
                },
            },
            fattura_elettronica_body: vec![fsm10::FatturaElettronicaBodyType {
                content: fsm10::FatturaElettronicaBodyTypeContent {
                    dati_generali: fsm10::DatiGeneraliType {
                        content: fsm10::DatiGeneraliTypeContent {
                            dati_generali_documento: fsm10::DatiGeneraliDocumentoType {
                                content: fsm10::DatiGeneraliDocumentoTypeContent {
                                    tipo_documento: rng
                                        .pick(&[
                                            fsm10::TipoDocumentoType::Td07,
                                            fsm10::TipoDocumentoType::Td08,
                                            fsm10::TipoDocumentoType::Td09,
                                        ])
                                        .clone(),
                                    divisa: "EUR".to_string(),
                                    data: date(&mut rng),
                                    numero: format!("{:0>6}", rng.below(1_000_000)),
                                    bollo_virtuale: if rng.bool() {
                                        Some(fsm10::BolloVirtualeType::Si)
                                    } else {
                                        None
                                    },
                                },
                            },
                            dati_fattura_rettificata: None,
                        },
                    },
                    dati_beni_servizi: vec![fsm10::DatiBeniServiziType {
                        content: fsm10::DatiBeniServiziTypeContent {
                            descrizione: "Servizio di consulenza".to_string(),
                            importo: amount(&mut rng),
                            dati_iva: fsm10::DatiIvaType {
                                content: fsm10::DatiIvaTypeContent {
                                    imposta: Some(amount(&mut rng)),
                                    aliquota: Some(rate(&mut rng)),
                                },
                            },
                            natura: if rng.bool() {
                                Some(
                                    rng.pick(&[fsm10::NaturaType::N1, fsm10::NaturaType::N2])
                                        .clone(),
                                )
                            } else {
                                None
                            },
                            riferimento_normativo: None,
                        },
                    }],
                    allegati: vec![],
                },
            }],
            signature: None,
        },
    }
}

/// A syntactically complete `ds:Signature`, carried by the seven SDI message
/// types that make it mandatory.
pub(crate) fn signature() -> sdi::Signature {
    sdi::SignatureType {
        id: None,
        content: sdi::SignatureTypeContent {
            signed_info: sdi::SignedInfoType {
                id: Some("sig".to_string()),
                content: sdi::SignedInfoTypeContent {
                    canonicalization_method: sdi::CanonicalizationMethodType {
                        algorithm: "http://www.w3.org/TR/2001/REC-xml-c14n-20010315#WithComments"
                            .to_string(),
                    },
                    signature_method: sdi::SignatureMethodType {
                        algorithm: "http://www.w3.org/2000/09/xmldsig#rsa-sha1".to_string(),
                        content: sdi::SignatureMethodTypeContent {
                            hmac_output_length: None,
                        },
                    },
                    reference: vec![sdi::ReferenceType {
                        id: None,
                        uri: Some("#-".to_string()),
                        type_: None,
                        content: sdi::ReferenceTypeContent {
                            transforms: None,
                            digest_method: sdi::DigestMethodType {
                                algorithm: "http://www.w3.org/2000/09/xmldsig#sha1".to_string(),
                            },
                            digest_value: Base64String("QVFBQVFBVQ==".to_string()),
                        },
                    }],
                },
            },
            signature_value: sdi::SignatureValueType {
                id: None,
                content: Base64String("c2lnbmF0dXJl".to_string()),
            },
            key_info: Some(sdi::KeyInfoType {
                id: None,
                content: vec![sdi::KeyInfoTypeContent::KeyName(
                    "signer@provider.it".to_string(),
                )],
            }),
            object: vec![],
        },
    }
}

pub(crate) fn identificativo(rng: &mut Rng) -> i32 {
    i32::try_from(rng.below(100_000) + 1).expect("fits an i32")
}

pub(crate) fn nome_file(rng: &mut Rng) -> String {
    format!("IT01234567890_{:0>5}.xml", rng.below(100_000) + 1)
}

pub(crate) fn message_id(rng: &mut Rng) -> String {
    format!("{:0>6}", rng.below(1_000_000) + 1)
}

pub(crate) fn riferimento_fattura(rng: &mut Rng) -> Option<sdi::RiferimentoFatturaType> {
    if rng.bool() {
        None
    } else {
        Some(sdi::RiferimentoFatturaType {
            content: sdi::RiferimentoFatturaTypeContent {
                numero_fattura: format!("{:0>6}", rng.below(1_000_000)),
                anno_fattura: 2020 + usize::try_from(rng.below(7)).expect("fits a usize"),
                posizione_fattura: None,
            },
        })
    }
}

pub(crate) fn destinatario(rng: &mut Rng) -> sdi::DestinatarioType {
    sdi::DestinatarioType {
        content: sdi::DestinatarioTypeContent {
            codice: codice_destinatario(rng),
            descrizione: "Agenzia delle Entrate".to_string(),
        },
    }
}

/// An `MT` metadata message.
pub(crate) fn mt_from_seed(seed: u64) -> MetadatiInvioFile {
    let mut rng = Rng::new(seed);
    MetadatiInvioFile {
        versione: "1.0".to_string(),
        content: sdi::MetadatiInvioFileTypeContent {
            identificativo_sd_i: identificativo(&mut rng),
            nome_file: nome_file(&mut rng),
            codice_destinatario: codice_destinatario(&mut rng),
            formato: rng.pick(&["SDI10", "FPR12", "FPA12"]).to_string(),
            tentativi_invio: i32::try_from(rng.below(3) + 1).expect("fits an i32"),
            message_id: message_id(&mut rng),
            note: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
        },
    }
}

/// An `EC` recipient outcome message, without a signature.
pub(crate) fn ec_from_seed(seed: u64) -> NotificaEsitoCommittente {
    let mut rng = Rng::new(seed);
    NotificaEsitoCommittente {
        versione: "1.0".to_string(),
        content: sdi::NotificaEsitoCommittenteTypeContent {
            identificativo_sd_i: identificativo(&mut rng),
            riferimento_fattura: riferimento_fattura(&mut rng),
            esito: rng
                .pick(&[
                    sdi::EsitoCommittenteType::Ec01,
                    sdi::EsitoCommittenteType::Ec02,
                ])
                .clone(),
            descrizione: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
            message_id_committente: if rng.bool() {
                Some(message_id(&mut rng))
            } else {
                None
            },
            signature: None,
        },
    }
}

/// An `AT` transmission attestation, with a mandatory `ds:Signature`.
pub(crate) fn at_from_seed(seed: u64) -> AttestazioneTrasmissioneFattura {
    let mut rng = Rng::new(seed);
    AttestazioneTrasmissioneFattura {
        versione: "1.0".to_string(),
        content: sdi::AttestazioneTrasmissioneFatturaTypeContent {
            identificativo_sd_i: identificativo(&mut rng),
            nome_file: nome_file(&mut rng),
            data_ora_ricezione: datetime(&mut rng),
            riferimento_archivio: if rng.bool() {
                Some(sdi::RiferimentoArchivioType {
                    content: sdi::RiferimentoArchivioTypeContent {
                        identificativo_sd_i: identificativo(&mut rng),
                        nome_file: nome_file(&mut rng),
                    },
                })
            } else {
                None
            },
            destinatario: destinatario(&mut rng),
            message_id: message_id(&mut rng),
            pec_message_id: if rng.bool() {
                Some(message_id(&mut rng))
            } else {
                None
            },
            note: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
            hash_file_originale: format!("{:0>32}", rng.below(u64::from(u32::MAX))),
            signature: signature(),
        },
    }
}

/// A `DT` deadline notice, with a mandatory `ds:Signature`.
pub(crate) fn dt_from_seed(seed: u64) -> NotificaDecorrenzaTermini {
    let mut rng = Rng::new(seed);
    NotificaDecorrenzaTermini {
        versione: "1.0".to_string(),
        intermediario_con_duplice_ruolo: if rng.bool() {
            Some("Si".to_string())
        } else {
            None
        },
        content: sdi::NotificaDecorrenzaTerminiTypeContent {
            identificativo_sd_i: identificativo(&mut rng),
            riferimento_fattura: riferimento_fattura(&mut rng),
            nome_file: nome_file(&mut rng),
            descrizione: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
            message_id: message_id(&mut rng),
            pec_message_id: if rng.bool() {
                Some(message_id(&mut rng))
            } else {
                None
            },
            note: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
            signature: signature(),
        },
    }
}

/// An `MC` failed delivery notification, with a mandatory `ds:Signature`.
pub(crate) fn mc_from_seed(seed: u64) -> NotificaMancataConsegna {
    let mut rng = Rng::new(seed);
    NotificaMancataConsegna {
        versione: "1.0".to_string(),
        content: sdi::NotificaMancataConsegnaTypeContent {
            identificativo_sd_i: identificativo(&mut rng),
            nome_file: nome_file(&mut rng),
            data_ora_ricezione: datetime(&mut rng),
            riferimento_archivio: if rng.bool() {
                Some(sdi::RiferimentoArchivioType {
                    content: sdi::RiferimentoArchivioTypeContent {
                        identificativo_sd_i: identificativo(&mut rng),
                        nome_file: nome_file(&mut rng),
                    },
                })
            } else {
                None
            },
            descrizione: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
            message_id: message_id(&mut rng),
            pec_message_id: if rng.bool() {
                Some(message_id(&mut rng))
            } else {
                None
            },
            note: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
            signature: signature(),
        },
    }
}

/// An `NE` sender outcome notification, with a mandatory `ds:Signature`.
pub(crate) fn ne_from_seed(seed: u64) -> NotificaEsito {
    let mut rng = Rng::new(seed);
    NotificaEsito {
        versione: "1.0".to_string(),
        intermediario_con_duplice_ruolo: if rng.bool() {
            Some("Si".to_string())
        } else {
            None
        },
        content: sdi::NotificaEsitoTypeContent {
            identificativo_sd_i: identificativo(&mut rng),
            nome_file: nome_file(&mut rng),
            esito_committente: ec_from_seed(seed),
            message_id: message_id(&mut rng),
            pec_message_id: if rng.bool() {
                Some(message_id(&mut rng))
            } else {
                None
            },
            note: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
            signature: signature(),
        },
    }
}

/// An `NS` rejection notification with one error, with a mandatory
/// `ds:Signature`.
pub(crate) fn ns_from_seed(seed: u64) -> NotificaScarto {
    let mut rng = Rng::new(seed);
    NotificaScarto {
        versione: "1.0".to_string(),
        content: sdi::NotificaScartoTypeContent {
            identificativo_sd_i: identificativo(&mut rng),
            nome_file: nome_file(&mut rng),
            data_ora_ricezione: datetime(&mut rng),
            riferimento_archivio: if rng.bool() {
                Some(sdi::RiferimentoArchivioType {
                    content: sdi::RiferimentoArchivioTypeContent {
                        identificativo_sd_i: identificativo(&mut rng),
                        nome_file: nome_file(&mut rng),
                    },
                })
            } else {
                None
            },
            lista_errori: sdi::ListaErroriType {
                content: sdi::ListaErroriTypeContent {
                    errore: vec![sdi::ErroreType {
                        content: sdi::ErroreTypeContent {
                            codice: "010101".to_string(),
                            descrizione: "Il file non e conforme al formato".to_string(),
                        },
                    }],
                },
            },
            message_id: message_id(&mut rng),
            pec_message_id: if rng.bool() {
                Some(message_id(&mut rng))
            } else {
                None
            },
            note: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
            signature: signature(),
        },
    }
}

/// An `RC` delivery receipt, with a mandatory `ds:Signature`.
pub(crate) fn rc_from_seed(seed: u64) -> RicevutaConsegna {
    let mut rng = Rng::new(seed);
    RicevutaConsegna {
        versione: "1.0".to_string(),
        intermediario_con_duplice_ruolo: if rng.bool() {
            Some("Si".to_string())
        } else {
            None
        },
        content: sdi::RicevutaConsegnaTypeContent {
            identificativo_sd_i: identificativo(&mut rng),
            nome_file: nome_file(&mut rng),
            data_ora_ricezione: datetime(&mut rng),
            data_ora_consegna: datetime(&mut rng),
            destinatario: destinatario(&mut rng),
            riferimento_archivio: if rng.bool() {
                Some(sdi::RiferimentoArchivioType {
                    content: sdi::RiferimentoArchivioTypeContent {
                        identificativo_sd_i: identificativo(&mut rng),
                        nome_file: nome_file(&mut rng),
                    },
                })
            } else {
                None
            },
            message_id: message_id(&mut rng),
            pec_message_id: if rng.bool() {
                Some(message_id(&mut rng))
            } else {
                None
            },
            note: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
            signature: signature(),
        },
    }
}

/// An `SE` recipient outcome rejection, with a mandatory `ds:Signature`.
pub(crate) fn se_from_seed(seed: u64) -> ScartoEsitoCommittente {
    let mut rng = Rng::new(seed);
    ScartoEsitoCommittente {
        versione: "1.0".to_string(),
        content: sdi::ScartoEsitoCommittenteTypeContent {
            identificativo_sd_i: identificativo(&mut rng),
            riferimento_fattura: riferimento_fattura(&mut rng),
            scarto: rng
                .pick(&[sdi::ScartoType::En00, sdi::ScartoType::En01])
                .clone(),
            message_id: message_id(&mut rng),
            message_id_committente: if rng.bool() {
                Some(message_id(&mut rng))
            } else {
                None
            },
            pec_message_id: if rng.bool() {
                Some(message_id(&mut rng))
            } else {
                None
            },
            note: if rng.bool() {
                Some("Esempio".to_string())
            } else {
                None
            },
            signature: signature(),
        },
    }
}
