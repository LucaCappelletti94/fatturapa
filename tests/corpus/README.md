# Real world corpus

Invoice and signed file samples copied from the test suites of nineteen
independent `FatturaPA` implementations, where the licence allows
redistribution. `tests/corpus.rs` reads every file, validates the `XML`
against the official schemas of `spec/` with `xmllint`, and round-trips every
file that validates. Every readable invoice also goes through `validate` or
`validate_simplified`.

The files are kept as released by their upstream suites, including the
invalid ones. The corpus is not regenerated, a new upstream release is a new
corpus.

## Sources and licences

Each source directory carries the upstream `LICENSE` file next to the copied files. The corpus is kept in the repository only and is excluded from the published crate.

| Directory | Upstream suite | Licence | Copied files |
| --- | --- | --- | --- |
| `descala/` | [descala/xsd-validator](https://github.com/descala/xsd-validator) | MIT | `Semplificata_FSM10.xml` |
| `eddydev-parser/` | [EddyDevProject/fatturapa-parser-php](https://github.com/EddyDevProject/fatturapa-parser-php) | MIT | `fattura*.xml`, three files |
| `fatturaelettronica-net/` | [FatturaElettronica/FatturaElettronica.NET](https://github.com/FatturaElettronica/FatturaElettronica.NET) | MIT style, (c) 2017 Nicola Iarocci, Gestionali Amica and contributors | `Samples/*.xml`, `Samples/*.p7m` |
| `fatturaelettronica-php/` | [fatturaelettronicaphp/FatturaElettronica](https://github.com/fatturaelettronicaphp/FatturaElettronica) | MIT | `tests/*.xml`, `tests/*.p7m`, `tests/semplificata/*.xml` |
| `fatturapa-php-sdk/` | [italia/fatturapa-php-sdk](https://github.com/italia/fatturapa-php-sdk) | BSD 3-Clause | `tests/.../IT01234567890_FPA02.xml` |
| `fexplorer/` | [EtheaDev/FExplorer](https://github.com/EtheaDev/FExplorer) | Apache 2.0 | `IT02408280960_FPA01.xml` |
| `gobl-sdi/` | [invopop/gobl.it.sdi](https://github.com/invopop/gobl.it.sdi) | Apache 2.0 | `fatturapa.gobl/*.xml`, `gobl.fatturapa/out/*.xml` |
| `horstoeko-invoicesuite/` | [horstoeko/invoicesuite](https://github.com/horstoeko/invoicesuite) | MIT | `tests/.../02_technical_xml_fatturapa.xml` |
| `json2fatturapaxml/` | [Palmabit-IT/json2fatturapaxml](https://github.com/Palmabit-IT/json2fatturapaxml) | MIT | `test/fixtures/*.xml`, three files |
| `kamerozkan/` | [kamerozkan/italy-fatturapa-validator-sample](https://github.com/kamerozkan/italy-fatturapa-validator-sample) | MIT | `accepted_fpr12.xml`, `rejected_arithmetic_fpr12.xml` |
| `marco-mastella/` | [marco-mastella/fatturapa](https://github.com/marco-mastella/fatturapa) | EUPL 1.2 | `xmlInvoices/*.xml`, `xmlInvalid/*.xml` |
| `mcp-fatturapa/` | [MaurizioLisanti/fatturapa-mcp-server](https://github.com/MaurizioLisanti/fatturapa-mcp-server) | MIT | `test/fixtures/*.xml`, three files |
| `openfatture/` | [gianlucamazza/openfatture](https://github.com/gianlucamazza/openfatture) | MIT | `tests/.../*.xml`, three files |
| `phax-fatturapa/` | [phax/ph-fatturapa](https://github.com/phax/ph-fatturapa) | Apache 2.0 | `tests/{1.1,120,122}/*.xml`, ten files |
| `python-a38/` | [Truelite/python-a38](https://github.com/Truelite/python-a38) | Apache 2.0 | `tests/data/*.xml` |
| `siboxd-model/` | [siboxd/fatturapa-model](https://github.com/siboxd/fatturapa-model) | MIT | `test/invoices/*.xml`, seven files |
| `simevo-json/` | [simevo/fattura-elettronica-json](https://github.com/simevo/fattura-elettronica-json) | BSD 3-Clause | `tests/{invalid,random,samples}/*.xml` |
| `slamdunk/` | [Slamdunk/php-validatore-fattura-elettronica](https://github.com/Slamdunk/php-validatore-fattura-elettronica) | MIT | `tests/*.xml`, nine files |
| `zugferd-corpus/` | [ZUGFeRD/corpus](https://github.com/ZUGFeRD/corpus) | Apache 2.0 | `eigor/valid/*.xml`, `official/valid/*.xml` |

All the licences permit redistribution of the sample files. The sixteen new
sources were copied at the commits `45b56827`, `fbd03966`, `d6b91a0b`,
`4e3b62e1`, `ab6bb846`, `c9da86ea`, `ecf8ec20`, `aceda310`, `c3d3f7fe`,
`59e33334`, `3f91becd`, `8eb300f7`, `2a4427de`, `1d70ec55`, `07f86a9c` and
`82e886fb` in the order of the table.

Suites examined and not copied, with the reason. `AGPL` licences that are
incompatible with redistribution, `italia/fatturapa-testsdi`,
`italia/fatturapa-magento`, `gigidn/...`, `provbz/...`, `Micronaet/...`,
`sidis405/...`, `consiglionazionale/...`, `ktecsrl/PyFePA`, the `AGPL`
`odoo` and `frappe` connectors. No usable licence, `italia/fatturapa-python`,
`s2software/fatturapa`, `link-it/govfat`, `link-it/govway`,
`cfrancescop/openspcoop`, `DelphiClubItalia/...`, `advinser/...`,
`walter-arrighetti/...`, `AgID/EeISI`, `erpreciso/fpa-reader`, `lmillucci/...`,
`rives1/...`, `Bruner4/...`, `orjn/...`, `darkrei08/...`. Byte duplicates of
files already in the corpus or in `spec/samples/`, `azuki-beans/fatturapa` and
`FatturaElettronica/FatturaElettronica.Extensions`. Templates only,
`Solede-SA/...`. The `simevo` `malicious/` directory is a set of `XML` bombs
rather than invoices and is not copied. `fexplorer/non_fattura.xml` is a
Delphi `TISFunction` document, the `kamerozkan` `UBL` file is not `FatturaPA`,
and `slamdunk/ok_semplificata` is a placeholder comment.

## Outcomes

One hundred and seventy three files. One hundred and three read, validate and
round-trip, including the two signed files unwrapped from their `.p7m`
wrapper. Sixty seven fail the `XSD` validation of `spec/`, listed below with
the reason `xmllint` reports. Three signed files do not unwrap at all, because
their `CMS` structure is damaged, and that is what their upstream suite
exercises them for.

Thirty seven of the round-tripped invoices carry a `Signature` element. They
round-trip equal, but the written bytes are not `XSD`-validated, because the
upstream `xsd-parser` loses the `ds:` namespace of the element on
serialisation (Bergmann89/xsd-parser#308).

### Files that fail the `XSD` validation

| File | Reason |
| --- | --- |
| `fatturaelettronica-net/IT01234567890_FPA02.xml` | `Percentuale` 10.0 does not match the two decimal pattern, a v1.0 sample |
| `fatturaelettronica-net/IT01234567890_FPR02.xml` | `Percentuale` 10.0 does not match the two decimal pattern, a v1.0 sample |
| `fatturaelettronica-net/IT02182030391_32.xml` | v1.1 `DatiRitenuta` element, absent from the v1.2 sequence |
| `fatturaelettronica-net/IT02182030391_32.windows-1252.xml` | v1.1 `DatiRitenuta` element, absent from the v1.2 sequence |
| `fatturaelettronica-php/IT01234567890_11001*.xml` | v1.0 namespace, no global element in the v1.2 schema, seven files |
| `fatturaelettronica-php/IT01234567890_11002.xml` | v1.0 namespace, no global element in the v1.2 schema |
| `fatturaelettronica-php/IT01234567890_21101.xml` | v1.0 namespace, no global element in the v1.2 schema |
| `fatturapa-php-sdk/IT01234567890_FPA02.xml` | `DataScadenzaPagamento` 2017-02-30 is not a valid `xs:date` |
| `gobl-sdi/fatturapa.gobl/acube_test.xml` | `CodiceDestinatario` where `IdTrasmittente` is expected |
| `gobl-sdi/fatturapa.gobl/invoice-fund-contribution-mixed-retention.xml` | `IstitutoFinanziario` absent from the v1.2 sequence |
| `gobl-sdi/fatturapa.gobl/invoice-windows1252.xml` | `CodiceArticolo` without the required `CodiceValore` |
| `json2fatturapaxml/IT03469550986_PR002.xml` | `Nome` where `Titolo` or `CodEORI` is expected |
| `marco-mastella/xmlInvalid/IT01234567890_FPA02.xml` | `versione` FPC12 not in the set |
| `marco-mastella/xmlInvalid/IT01234567890_FPR02.xml` | `FatturaElettronicaBody` where `FatturaElettronicaHeader` is expected |
| `marco-mastella/xmlInvalid/IT01234567890_FPR03.xml` | missing the required `FatturaElettronicaBody` |
| `mcp-fatturapa/invalid.xml` | premature end of data in `IdTrasmittente` |
| `mcp-fatturapa/valid_v13.xml` | v1.3 namespace, no global element in the v1.2 schema |
| `openfatture/NS_2025-003.xml` | unqualified `NotificaScarto` root, no global element |
| `openfatture/RC_2025-002.xml` | unqualified `RicevutaConsegna` root, no global element |
| `phax-fatturapa/1.1/IT01234567890_1100*.xml` | v1.1 namespace, no global element in the v1.2 schema, four files |
| `phax-fatturapa/120/bad/IT01234567890_FPA01.xml` | missing the required `FatturaElettronicaBody` |
| `python-a38/dati_trasporto.xml` | v1.1 `DatiRitenuta` element, absent from the v1.2 sequence |
| `python-a38/unicode.xml` | missing child elements in `DatiAnagrafici` |
| `simevo-json/invalid/malformed.xml` | prefix `p` on `FatturaElettronica` not defined |
| `simevo-json/invalid/missing_*.xml` | one required element removed per file, eighteen files |
| `simevo-json/random/00.xml` to `08.xml` | `Nome` or `Cognome` outside the allowed set, nine files |
| `simevo-json/random/09.xml` | `RegimeFiscale` RF82 not in the set |
| `slamdunk/invalid_IT01234567890_11111_EC_001.xml` | empty `IdentificativoSdI`, not a valid atomic value |
| `slamdunk/invalid_xml_tags.xml` | prefix `ns2` on `FatturaElettronica` not defined |
| `slamdunk/invalid_xsd_content.xml` | `IdPaese` ITALIA does not match the pattern |
| `slamdunk/invalid_xsd_structure_compliance.xml` | `IdTrasmittente` without `IdCodice` |
| `zugferd-corpus/eigor/valid/con-ritenuta-acconto-e-cassa-previdenziale.xml` | premature end of data in `FatturaElettronica` |
| `zugferd-corpus/official/valid/IT01234567890_FPR02.xml` | `ContattiTrasmittente` not expected, the defect of the official v1.2 sample |

### Signed files that do not unwrap

| File | State |
| --- | --- |
| `fatturaelettronica-net/IT02182030391_31_tampered.Base64.xml.p7m` | outer `SEQUENCE` length damaged |
| `fatturaelettronica-net/IT02182030391_31_tampered.xml.p7m` | nested `SEQUENCE` length damaged |
| `fatturaelettronica-php/invalid.xml.p7m` | the trailing `00 00` end of contents is `00 01` |

The remaining `FatturaElettronica.NET` signed files exercise the `BER`
indefinite length form that the .NET runtime writes, and the signer that
splits the `eContent` into 1000 byte chunks under a constructed `OCTET
STRING`. `fatturaelettronica-php/IT00484960588_ERKHK.xml.p7m` is a signed
invoice with a 1.4 MB attachment.

### Files on which `validate` reports violations

Every reported violation is a true SDI finding about the file, checked against
the Elenco Controlli v2.0 text, and against the Allegato A 1.9.1 operative
specification for 00471, whose version 2.0 list predates the extension to
`TD01`, `TD02`, `TD03`, `TD06`, `TD24`, `TD25` and `TD28`. Every other
readable invoice is clean.

| File | Codes | What the file does |
| --- | --- | --- |
| `fatturaelettronica-php/IT01234567899_000sq.xml` | 00421 twice, 00445 twice | a riepilogo `Imposta` off by exactly one cent, and the generic nature `N2` on a line and a riepilogo |
| `gobl-sdi/fatturapa.gobl/invoice-b2g.xml` | 00427 | `FPA12` with a seven character `CodiceDestinatario` |
| `gobl-sdi/gobl.fatturapa/out/invoice-b2g.xml` | 00427 | `FPA12` with a seven character `CodiceDestinatario` |
| `gobl-sdi/gobl.fatturapa/out/invoice-simple-with-credits.xml` | 00423 | a credit line with positive amounts and a negative `PrezzoTotale` |
| `horstoeko-invoicesuite/02_technical_xml_fatturapa.xml` | 00423 | a line `ScontoMaggiorazione` with both `Percentuale` and `Importo` |
| `json2fatturapaxml/IT03469550986_PA001.xml` | 00471 | a `TD01` with a seller and a buyer sharing the same `IdFiscaleIVA` |
| `json2fatturapaxml/IT03469550986_PR001.xml` | 00471 | a `TD01` with a seller and a buyer sharing the same `IdFiscaleIVA` |
| `kamerozkan/rejected_arithmetic_fpr12.xml` | 00421, 00422 | riepilogo `Imposta` and `ImponibileImporto` off from the recomputed values |
| `marco-mastella/xmlInvoices/IT01234567890_FPR03.xml` | 00421, 00422 | riepilogo `Imposta` off by one cent and `ImponibileImporto` off by two |
| `phax-fatturapa/122/bad/IT01234567890_FPR01_TD29_RF20.xml` | 00475 | `TD29` without the buyer `IdFiscaleIVA` |
| `phax-fatturapa/122/good/IT01234567890_FPR02.xml` | 00421, 00422 | riepilogo `Imposta` off by one cent and `ImponibileImporto` off by two |
| `siboxd-model/IT01234567890_FPR03.xml` | 00421, 00422 | riepilogo `Imposta` off by one cent and `ImponibileImporto` off by two |
| `siboxd-model/IT01234567890_FPR04.xml` | 00421, 00422 | riepilogo `Imposta` off by one cent and `ImponibileImporto` off by two |
| `simevo-json/samples/IT01234567890_FPR03.xml` | 00421, 00422 | riepilogo `Imposta` off by one cent and `ImponibileImporto` off by two |
| `zugferd-corpus/eigor/valid/A10-Licenses-CreditNote.xml` | 00476 | seller and buyer `IdPaese` both `NL` |
| `zugferd-corpus/eigor/valid/A10-Licenses.xml` | 00476 | seller and buyer `IdPaese` both `NL` |
| `zugferd-corpus/eigor/valid/A4-Subscription.xml` | 00476 | seller and buyer `IdPaese` both `NL` |
| `zugferd-corpus/eigor/valid/fattpa-creditnote.to-ublcn-working-example.xml` | 00476 | seller and buyer `IdPaese` both `NL` |
