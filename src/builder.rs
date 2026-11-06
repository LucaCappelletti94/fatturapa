//! A builder for the ordinary `FatturaPA` invoice over the generated types.
//!
//! The builder covers the common case, one seller issuing one invoice with
//! one or more `FatturaElettronicaBody` blocks. It computes the line
//! `PrezzoTotale` per the 00423 rule, the `DatiRiepilogo` blocks and the
//! `ImportoTotaleDocumento` per the rounding rules of the specification,
//! and `build` runs `validate::validate` before returning the document.
//!
//! The builder is a convenience layer over the generated types. The caller
//! can still edit the returned `FatturaElettronica`, or assemble the
//! generated structs directly, for the elements the builder does not set,
//! such as `DatiPagamento`, `DatiRitenuta` and the riepilogo
//! `EsigibilitaIVA`. A foreign buyer, `IdPaese` different from `IT`, gets
//! the `CodiceDestinatario` `XXXXXXX` automatically; a domestic buyer
//! needs an explicit code.

use crate::FatturaElettronica;
use crate::amount::Amount;
use crate::date::Date;
use crate::generated::fpr12::{
    AnagraficaContent60Type, AnagraficaContent60TypeContent, AnagraficaContent61Type,
    AnagraficaContent61TypeContent, AnagraficaContent62Type, AnagraficaContent62TypeContent,
    AnagraficaType, AnagraficaTypeContent, BolloVirtualeType, CedentePrestatoreType,
    CedentePrestatoreTypeContent, CessionarioCommittenteType, CessionarioCommittenteTypeContent,
    DatiAnagraficiCedenteType, DatiAnagraficiCedenteTypeContent, DatiAnagraficiCessionarioType,
    DatiAnagraficiCessionarioTypeContent, DatiBeniServiziType, DatiBeniServiziTypeContent,
    DatiBolloType, DatiBolloTypeContent, DatiGeneraliDocumentoType,
    DatiGeneraliDocumentoTypeContent, DatiGeneraliType, DatiGeneraliTypeContent, DatiRiepilogoType,
    DatiRiepilogoTypeContent, DatiTrasmissioneType, DatiTrasmissioneTypeContent,
    DettaglioLineeType, DettaglioLineeTypeContent, FatturaElettronicaBodyType,
    FatturaElettronicaBodyTypeContent, FatturaElettronicaHeaderType,
    FatturaElettronicaHeaderTypeContent, FatturaElettronicaTypeContent, FormatoTrasmissioneType,
    IdFiscaleType, IdFiscaleTypeContent, IndirizzoType, IndirizzoTypeContent, NaturaType,
    RegimeFiscaleType, RitenutaType, ScontoMaggiorazioneType, ScontoMaggiorazioneTypeContent,
    TipoDocumentoType, TipoScontoMaggiorazioneType,
};
use crate::validate::{self, Violation};
use rust_decimal::Decimal;

/// The `CodiceDestinatario` the SDI assigns to a non-resident buyer.
pub const FOREIGN_CODICE_DESTINATARIO: &str = "XXXXXXX";

/// The 77.47 EUR stamp duty threshold of the DPR 642/72.
const BOLLO_THRESHOLD: Decimal = Decimal::from_parts(7747, 0, 0, false, 2);

/// The 2.00 EUR amount of the virtual stamp duty.
const BOLLO_IMPORT: Decimal = Decimal::from_parts(200, 0, 0, false, 2);

/// What `Invoice::build` refuses when the document cannot be assembled.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BuildError {
    /// A required field the caller did not set, one entry per field.
    #[error("missing required field(s): {}", .0.join(", "))]
    Missing(Vec<String>),
    /// The assembled document fails one or more SDI checks.
    #[error("the document fails {} SDI check(s)", .0.len())]
    Violations(Vec<Violation>),
}

/// A builder for an ordinary invoice, one seller and one or more
/// `FatturaElettronicaBody` blocks.
#[derive(Clone, Debug)]
pub struct Invoice {
    /// The transmission format, `FPR12` or `FPA12`.
    formato: FormatoTrasmissioneType,
    /// The `ProgressivoInvio`, four digits.
    progressivo_invio: String,
    /// The seller, which also fills `IdTrasmittente`.
    seller: CedentePrestatoreType,
    /// The buyer.
    buyer: CessionarioCommittenteType,
    /// The `CodiceDestinatario`, set explicitly or defaulted for a foreign
    /// buyer.
    codice_destinatario: Option<String>,
    /// The `FatturaElettronicaBody` blocks, in document order.
    bodies: Vec<InvoiceBody>,
}

impl Invoice {
    /// Starts the builder. `formato` sets both the root `versione`
    /// attribute and the `FormatoTrasmissione` element, and the seller VAT
    /// number fills `IdTrasmittente`.
    pub fn new(
        formato: FormatoTrasmissioneType,
        progressivo_invio: impl Into<String>,
        seller: CedentePrestatoreType,
        buyer: CessionarioCommittenteType,
    ) -> Self {
        Self {
            formato,
            progressivo_invio: progressivo_invio.into(),
            seller,
            buyer,
            codice_destinatario: None,
            bodies: Vec::new(),
        }
    }

    /// Sets the `CodiceDestinatario` for a domestic buyer.
    #[must_use]
    pub fn codice_destinatario(mut self, codice: impl Into<String>) -> Self {
        self.codice_destinatario = Some(codice.into());
        self
    }

    /// Adds one `FatturaElettronicaBody`.
    #[must_use]
    pub fn body(mut self, body: InvoiceBody) -> Self {
        self.bodies.push(body);
        self
    }

    /// Assembles the document, computes the totals and the riepilogo and
    /// runs `validate::validate`.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the missing fields when a required one is not
    /// set, or with every SDI violation when the assembled document fails
    /// a check.
    pub fn build(self) -> Result<FatturaElettronica, BuildError> {
        let buyer_iva = self
            .buyer
            .content
            .dati_anagrafici
            .content
            .id_fiscale_iva
            .as_ref();
        let domestic = buyer_iva.is_none_or(|id| id.content.id_paese == "IT");
        let mut missing = Vec::new();
        let codice = self
            .codice_destinatario
            .or_else(|| (!domestic).then(|| FOREIGN_CODICE_DESTINATARIO.to_string()));
        let codice = codice.unwrap_or_else(|| {
            missing.push("codice_destinatario".to_string());
            String::new()
        });
        if !missing.is_empty() {
            return Err(BuildError::Missing(missing));
        }
        let formato = self.formato;
        let header = header(
            formato.clone(),
            self.progressivo_invio,
            codice,
            self.seller,
            self.buyer,
        );
        let bodies = self
            .bodies
            .into_iter()
            .map(assemble_body)
            .collect::<Vec<_>>();
        let doc = FatturaElettronica {
            versione: formato,
            sistema_emittente: None,
            content: FatturaElettronicaTypeContent {
                fattura_elettronica_header: header,
                fattura_elettronica_body: bodies,
                signature: None,
            },
        };
        let violations = validate::validate(&doc);
        if violations.is_empty() {
            Ok(doc)
        } else {
            Err(BuildError::Violations(violations))
        }
    }
}

/// The data of one `FatturaElettronicaBody`, before its totals are
/// computed.
#[derive(Clone, Debug)]
pub struct InvoiceBody {
    /// The `TipoDocumento`.
    tipo_documento: TipoDocumentoType,
    /// The `Divisa`, three letters.
    divisa: String,
    /// The `Data` of the document.
    data: Date,
    /// The `Numero` of the document.
    numero: String,
    /// The `Causale` texts.
    causali: Vec<String>,
    /// Whether the virtual stamp duty applies.
    bollo_virtuale: bool,
    /// The lines, in document order.
    lines: Vec<Line>,
    /// The `RiferimentoNormativo` of the riepilogo of each nature.
    riferimenti_normativi: Vec<(NaturaType, String)>,
}

impl InvoiceBody {
    /// Starts a body with its document type, currency, date and number.
    pub fn new(
        tipo_documento: TipoDocumentoType,
        divisa: impl Into<String>,
        data: Date,
        numero: impl Into<String>,
    ) -> Self {
        Self {
            tipo_documento,
            divisa: divisa.into(),
            data,
            numero: numero.into(),
            causali: Vec::new(),
            bollo_virtuale: false,
            lines: Vec::new(),
            riferimenti_normativi: Vec::new(),
        }
    }

    /// Adds a `Causale`.
    #[must_use]
    pub fn causale(mut self, causale: impl Into<String>) -> Self {
        self.causali.push(causale.into());
        self
    }

    /// Enables the virtual stamp duty. The 2.00 EUR amount is set when the
    /// computed total is above 77.47.
    #[must_use]
    pub const fn bollo_virtuale(mut self) -> Self {
        self.bollo_virtuale = true;
        self
    }

    /// Sets the `RiferimentoNormativo` written on the riepilogo of the lines
    /// with `natura`, such as `ART. 7-TER DPR 633/72` for `N2.1`. A later call
    /// for the same nature replaces the earlier text.
    #[must_use]
    pub fn riferimento_normativo(mut self, natura: NaturaType, text: impl Into<String>) -> Self {
        let text = text.into();
        match self
            .riferimenti_normativi
            .iter_mut()
            .find(|(n, _)| *n == natura)
        {
            Some(entry) => entry.1 = text,
            None => self.riferimenti_normativi.push((natura, text)),
        }
        self
    }

    /// Adds a line.
    #[must_use]
    pub fn line(mut self, line: Line) -> Self {
        self.lines.push(line);
        self
    }
}

/// One `DettaglioLinee`, before its `PrezzoTotale` is computed.
#[derive(Clone, Debug)]
pub struct Line {
    /// The `NumeroLinea`.
    numero_linea: i32,
    /// The `Descrizione`.
    descrizione: String,
    /// The `Quantita`, 1 when not set.
    quantita: Option<Amount>,
    /// The `UnitaMisura`.
    unita_misura: Option<String>,
    /// The `PrezzoUnitario`.
    prezzo_unitario: Amount,
    /// The `ScontoMaggiorazione` blocks.
    sconto_maggiorazione: Vec<ScontoMaggiorazioneType>,
    /// The `AliquotaIVA`, a percentage.
    aliquota_iva: Amount,
    /// The `Ritenuta`.
    ritenuta: Option<RitenutaType>,
    /// The `Natura` of the non-taxable operation.
    natura: Option<NaturaType>,
}

impl Line {
    /// Starts a line with its number, description, unit price and VAT rate.
    /// The quantity defaults to 1 when not set.
    pub fn new(
        numero_linea: i32,
        descrizione: impl Into<String>,
        prezzo_unitario: Amount,
        aliquota_iva: Amount,
    ) -> Self {
        Self {
            numero_linea,
            descrizione: descrizione.into(),
            quantita: None,
            unita_misura: None,
            prezzo_unitario,
            sconto_maggiorazione: Vec::new(),
            aliquota_iva,
            ritenuta: None,
            natura: None,
        }
    }

    /// Sets the `Quantita`.
    #[must_use]
    pub const fn quantita(mut self, quantita: Amount) -> Self {
        self.quantita = Some(quantita);
        self
    }

    /// Sets the `UnitaMisura`.
    #[must_use]
    pub fn unita_misura(mut self, unita: impl Into<String>) -> Self {
        self.unita_misura = Some(unita.into());
        self
    }

    /// Sets the `Natura` of the non-taxable operation.
    #[must_use]
    pub const fn natura(mut self, natura: NaturaType) -> Self {
        self.natura = Some(natura);
        self
    }

    /// Sets the `Ritenuta` to `SI`.
    #[must_use]
    pub const fn ritenuta(mut self) -> Self {
        self.ritenuta = Some(RitenutaType::Si);
        self
    }

    /// Adds a `ScontoMaggiorazione`.
    #[must_use]
    pub fn sconto_maggiorazione(mut self, sconto: ScontoMaggiorazioneType) -> Self {
        self.sconto_maggiorazione.push(sconto);
        self
    }

    /// Consumes the line into its generated content with a zero
    /// `PrezzoTotale`, which `assemble_body` overwrites.
    fn into_content(self) -> DettaglioLineeTypeContent {
        DettaglioLineeTypeContent {
            numero_linea: self.numero_linea,
            tipo_cessione_prestazione: None,
            codice_articolo: Vec::new(),
            descrizione: self.descrizione,
            quantita: self.quantita,
            unita_misura: self.unita_misura,
            data_inizio_periodo: None,
            data_fine_periodo: None,
            prezzo_unitario: self.prezzo_unitario,
            sconto_maggiorazione: self.sconto_maggiorazione,
            prezzo_totale: Amount(Decimal::ZERO),
            aliquota_iva: self.aliquota_iva,
            ritenuta: self.ritenuta,
            natura: self.natura,
            riferimento_amministrazione: None,
            altri_dati_gestionali: Vec::new(),
        }
    }
}

/// The header of the assembled invoice.
#[must_use]
fn header(
    formato: FormatoTrasmissioneType,
    progressivo_invio: String,
    codice_destinatario: String,
    seller: CedentePrestatoreType,
    buyer: CessionarioCommittenteType,
) -> FatturaElettronicaHeaderType {
    let id_trasmittente = seller
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .clone();
    FatturaElettronicaHeaderType {
        content: FatturaElettronicaHeaderTypeContent {
            dati_trasmissione: DatiTrasmissioneType {
                content: DatiTrasmissioneTypeContent {
                    id_trasmittente,
                    progressivo_invio,
                    formato_trasmissione: formato,
                    codice_destinatario,
                    contatti_trasmittente: None,
                    pec_destinatario: None,
                },
            },
            cedente_prestatore: seller,
            rappresentante_fiscale: None,
            cessionario_committente: buyer,
            terzo_intermediario_o_soggetto_emittente: None,
            soggetto_emittente: None,
        },
    }
}

/// Assembles one body, computing the line totals, the riepilogo and the
/// total amount of the document.
#[must_use]
fn assemble_body(body: InvoiceBody) -> FatturaElettronicaBodyType {
    let mut lines: Vec<DettaglioLineeType> = Vec::new();
    let mut groups: Vec<(Decimal, Option<NaturaType>, Decimal)> = Vec::new();
    for line in body.lines {
        let mut content = line.into_content();
        content.prezzo_totale = validate::line_total(&content);
        let key = (content.aliquota_iva.0, content.natura.clone());
        match groups
            .iter_mut()
            .find(|(a, n, _)| *a == key.0 && *n == key.1)
        {
            Some(group) => group.2 += content.prezzo_totale.0,
            None => groups.push((key.0, key.1, content.prezzo_totale.0)),
        }
        lines.push(DettaglioLineeType { content });
    }
    let mut riep: Vec<DatiRiepilogoType> = Vec::new();
    let mut totale = Decimal::ZERO;
    for (aliquota, natura, raw) in groups {
        let imponibile = validate::round_half_2(raw);
        let arrotondamento = imponibile - raw;
        let imposta = validate::round_half_2(aliquota * imponibile / Decimal::ONE_HUNDRED);
        totale += imponibile + imposta;
        let riferimento_normativo = natura.as_ref().and_then(|natura| {
            body.riferimenti_normativi
                .iter()
                .find(|(n, _)| n == natura)
                .map(|(_, text)| text.clone())
        });
        riep.push(DatiRiepilogoType {
            content: DatiRiepilogoTypeContent {
                aliquota_iva: Amount(aliquota),
                natura,
                spese_accessorie: None,
                arrotondamento: (arrotondamento != Decimal::ZERO).then_some(Amount(arrotondamento)),
                imponibile_importo: Amount(imponibile),
                imposta: Amount(imposta),
                esigibilita_iva: None,
                riferimento_normativo,
            },
        });
    }
    let importo_totale = validate::round_half_2(totale);
    let bollo = if body.bollo_virtuale {
        let importo = (importo_totale > BOLLO_THRESHOLD).then_some(Amount(BOLLO_IMPORT));
        Some(DatiBolloType {
            content: DatiBolloTypeContent {
                bollo_virtuale: BolloVirtualeType::Si,
                importo_bollo: importo,
            },
        })
    } else {
        None
    };
    FatturaElettronicaBodyType {
        content: FatturaElettronicaBodyTypeContent {
            dati_generali: DatiGeneraliType {
                content: DatiGeneraliTypeContent {
                    dati_generali_documento: DatiGeneraliDocumentoType {
                        content: DatiGeneraliDocumentoTypeContent {
                            tipo_documento: body.tipo_documento,
                            divisa: body.divisa,
                            data: body.data,
                            numero: body.numero,
                            dati_ritenuta: Vec::new(),
                            dati_bollo: bollo,
                            dati_cassa_previdenziale: Vec::new(),
                            sconto_maggiorazione: Vec::new(),
                            importo_totale_documento: Some(Amount(importo_totale)),
                            arrotondamento: None,
                            causale: body.causali,
                            art_73: None,
                        },
                    },
                    dati_ordine_acquisto: Vec::new(),
                    dati_contratto: Vec::new(),
                    dati_convenzione: Vec::new(),
                    dati_ricezione: Vec::new(),
                    dati_fatture_collegate: Vec::new(),
                    dati_sal: Vec::new(),
                    dati_ddt: Vec::new(),
                    dati_trasporto: None,
                    fattura_principale: None,
                },
            },
            dati_beni_servizi: DatiBeniServiziType {
                content: DatiBeniServiziTypeContent {
                    dettaglio_linee: lines,
                    dati_riepilogo: riep,
                },
            },
            dati_veicoli: None,
            dati_pagamento: Vec::new(),
            allegati: Vec::new(),
        },
    }
}

impl IdFiscaleType {
    /// Builds an `IdFiscaleIVA` from the two letter country code and the
    /// VAT or tax code.
    pub fn new(id_paese: impl Into<String>, id_codice: impl Into<String>) -> Self {
        Self {
            content: IdFiscaleTypeContent {
                id_paese: id_paese.into(),
                id_codice: id_codice.into(),
            },
        }
    }
}

impl IndirizzoType {
    /// Builds an address with the required elements. The caller fills
    /// `numero_civico` and `provincia` when needed.
    pub fn new(
        indirizzo: impl Into<String>,
        cap: impl Into<String>,
        comune: impl Into<String>,
        nazione: impl Into<String>,
    ) -> Self {
        Self {
            content: IndirizzoTypeContent {
                indirizzo: indirizzo.into(),
                numero_civico: None,
                cap: cap.into(),
                comune: comune.into(),
                provincia: None,
                nazione: nazione.into(),
            },
        }
    }
}

impl AnagraficaType {
    /// Builds an anagrafica identified by the company name.
    pub fn denominazione(denominazione: impl Into<String>) -> Self {
        Self {
            content: AnagraficaTypeContent {
                content_60: AnagraficaContent60Type {
                    content: AnagraficaContent60TypeContent::Content61(AnagraficaContent61Type {
                        content: AnagraficaContent61TypeContent {
                            denominazione: denominazione.into(),
                        },
                    }),
                },
                titolo: None,
                cod_eori: None,
            },
        }
    }

    /// Builds an anagrafica identified by the name and the surname.
    pub fn nome_cognome(nome: impl Into<String>, cognome: impl Into<String>) -> Self {
        Self {
            content: AnagraficaTypeContent {
                content_60: AnagraficaContent60Type {
                    content: AnagraficaContent60TypeContent::Content62(AnagraficaContent62Type {
                        content: AnagraficaContent62TypeContent {
                            nome: nome.into(),
                            cognome: cognome.into(),
                        },
                    }),
                },
                titolo: None,
                cod_eori: None,
            },
        }
    }
}

impl CedentePrestatoreType {
    /// Builds a seller with a VAT number, an anagrafica, a tax regime and
    /// its seat.
    #[must_use]
    pub const fn new(
        id_fiscale_iva: IdFiscaleType,
        anagrafica: AnagraficaType,
        regime_fiscale: RegimeFiscaleType,
        sede: IndirizzoType,
    ) -> Self {
        Self {
            content: CedentePrestatoreTypeContent {
                dati_anagrafici: DatiAnagraficiCedenteType {
                    content: DatiAnagraficiCedenteTypeContent {
                        id_fiscale_iva,
                        codice_fiscale: None,
                        anagrafica,
                        albo_professionale: None,
                        provincia_albo: None,
                        numero_iscrizione_albo: None,
                        data_iscrizione_albo: None,
                        regime_fiscale,
                    },
                },
                sede,
                stabile_organizzazione: None,
                iscrizione_rea: None,
                contatti: None,
                riferimento_amministrazione: None,
            },
        }
    }

    /// Sets the `CodiceFiscale` of the seller.
    #[must_use]
    pub fn codice_fiscale(mut self, codice: impl Into<String>) -> Self {
        self.content.dati_anagrafici.content.codice_fiscale = Some(codice.into());
        self
    }
}

impl CessionarioCommittenteType {
    /// Builds a buyer identified by the VAT number.
    #[must_use]
    pub const fn new(
        id_fiscale_iva: IdFiscaleType,
        anagrafica: AnagraficaType,
        sede: IndirizzoType,
    ) -> Self {
        Self {
            content: CessionarioCommittenteTypeContent {
                dati_anagrafici: DatiAnagraficiCessionarioType {
                    content: DatiAnagraficiCessionarioTypeContent {
                        id_fiscale_iva: Some(id_fiscale_iva),
                        codice_fiscale: None,
                        anagrafica,
                    },
                },
                sede,
                stabile_organizzazione: None,
                rappresentante_fiscale: None,
            },
        }
    }

    /// Builds a buyer identified only by the tax code, the common case for
    /// private individuals.
    pub fn codice_fiscale(
        codice: impl Into<String>,
        anagrafica: AnagraficaType,
        sede: IndirizzoType,
    ) -> Self {
        Self {
            content: CessionarioCommittenteTypeContent {
                dati_anagrafici: DatiAnagraficiCessionarioType {
                    content: DatiAnagraficiCessionarioTypeContent {
                        id_fiscale_iva: None,
                        codice_fiscale: Some(codice.into()),
                        anagrafica,
                    },
                },
                sede,
                stabile_organizzazione: None,
                rappresentante_fiscale: None,
            },
        }
    }
}

impl DatiBolloType {
    /// Builds a virtual stamp duty, with the amount when it applies.
    #[must_use]
    pub const fn virtuale(importo_bollo: Option<Amount>) -> Self {
        Self {
            content: DatiBolloTypeContent {
                bollo_virtuale: BolloVirtualeType::Si,
                importo_bollo,
            },
        }
    }
}

impl ScontoMaggiorazioneType {
    /// Builds a discount or markup expressed as a percentage.
    #[must_use]
    pub const fn percentuale(tipo: TipoScontoMaggiorazioneType, percentuale: Amount) -> Self {
        Self {
            content: ScontoMaggiorazioneTypeContent {
                tipo,
                percentuale: Some(percentuale),
                importo: None,
            },
        }
    }

    /// Builds a discount or markup expressed as an amount.
    #[must_use]
    pub const fn importo(tipo: TipoScontoMaggiorazioneType, importo: Amount) -> Self {
        Self {
            content: ScontoMaggiorazioneTypeContent {
                tipo,
                percentuale: None,
                importo: Some(importo),
            },
        }
    }
}
