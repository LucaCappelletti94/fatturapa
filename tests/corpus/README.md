# Real world corpus

Invoice and signed file samples copied from the test suites of twenty-nine
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
| `estratto/` | [carminelaluna/estratto](https://github.com/carminelaluna/estratto) | MIT | `*.xml`, nine files |
| `fatturaelettronica-net/` | [FatturaElettronica/FatturaElettronica.NET](https://github.com/FatturaElettronica/FatturaElettronica.NET) | MIT style, (c) 2017 Nicola Iarocci, Gestionali Amica and contributors | `Samples/*.xml`, `Samples/*.p7m` |
| `fatturaelettronica-php/` | [fatturaelettronicaphp/FatturaElettronica](https://github.com/fatturaelettronicaphp/FatturaElettronica) | MIT | `tests/*.xml`, `tests/*.p7m`, `tests/semplificata/*.xml` |
| `fatturapa-php-sdk/` | [italia/fatturapa-php-sdk](https://github.com/italia/fatturapa-php-sdk) | BSD 3-Clause | `tests/.../IT01234567890_FPA02.xml` |
| `fatture-elettroniche-explorer/` | [N1K1TAS95/fatture_elettroniche_explorer](https://github.com/N1K1TAS95/fatture_elettroniche_explorer) | MIT | `carburante/*.xml`, `casi-limite/*.xml`, `emesse/*.xml`, `ricevute/*.xml`, `firmate/*.p7m`, twenty eight files |
| `fexplorer/` | [EtheaDev/FExplorer](https://github.com/EtheaDev/FExplorer) | Apache 2.0 | `IT02408280960_FPA01.xml` |
| `gobl-sdi/` | [invopop/gobl.it.sdi](https://github.com/invopop/gobl.it.sdi) | Apache 2.0 | `fatturapa.gobl/*.xml`, `gobl.fatturapa/out/*.xml` |
| `horstoeko-invoicesuite/` | [horstoeko/invoicesuite](https://github.com/horstoeko/invoicesuite) | MIT | `tests/.../02_technical_xml_fatturapa.xml` |
| `invoice-extract/` | [sonoyumi/invoice-extract](https://github.com/sonoyumi/invoice-extract) | MIT | `*.xml`, `*.p7m`, seven files |
| `json2fatturapaxml/` | [Palmabit-IT/json2fatturapaxml](https://github.com/Palmabit-IT/json2fatturapaxml) | MIT | `test/fixtures/*.xml`, three files |
| `kamerozkan/` | [kamerozkan/italy-fatturapa-validator-sample](https://github.com/kamerozkan/italy-fatturapa-validator-sample) | MIT | `accepted_fpr12.xml`, `rejected_arithmetic_fpr12.xml` |
| `marco-mastella/` | [marco-mastella/fatturapa](https://github.com/marco-mastella/fatturapa) | EUPL 1.2 | `xmlInvoices/*.xml`, `xmlInvalid/*.xml` |
| `mcp-fatturapa/` | [MaurizioLisanti/fatturapa-mcp-server](https://github.com/MaurizioLisanti/fatturapa-mcp-server) | MIT | `test/fixtures/*.xml`, three files |
| `merge-invoices/` | [giosil/merge-invoices](https://github.com/giosil/merge-invoices) | Apache 2.0 | `*_invoice.xml`, `*_credit_note.xml`, six files |
| `odoo-fatturapa-viewer/` | [julioromano/odoo-fatturapa-viewer](https://github.com/julioromano/odoo-fatturapa-viewer) | Apache 2.0 | `test.xml`, `test-bad.xml`, `test-bad.xml.p7m` |
| `openfatture/` | [gianlucamazza/openfatture](https://github.com/gianlucamazza/openfatture) | MIT | `tests/.../*.xml`, three files |
| `p7m-apri/` | [azuki-beans/p7m-apri](https://github.com/azuki-beans/p7m-apri) | MIT | `*.p7m`, three files |
| `p7m-fattura-decoder/` | [controlaltjeff/p7m-fattura-decoder](https://github.com/controlaltjeff/p7m-fattura-decoder) | MIT | `minimal-valid.xml`, `test-invoice.p7m`, `truncated.p7m` |
| `phax-fatturapa/` | [phax/ph-fatturapa](https://github.com/phax/ph-fatturapa) | Apache 2.0 | `tests/{1.1,120,122}/*.xml`, ten files |
| `python-a38/` | [Truelite/python-a38](https://github.com/Truelite/python-a38) | Apache 2.0 | `tests/data/*.xml` |
| `siboxd-model/` | [siboxd/fatturapa-model](https://github.com/siboxd/fatturapa-model) | MIT | `test/invoices/*.xml`, seven files |
| `simevo-json/` | [simevo/fattura-elettronica-json](https://github.com/simevo/fattura-elettronica-json) | BSD 3-Clause | `tests/{invalid,random,samples}/*.xml` |
| `slamdunk/` | [Slamdunk/php-validatore-fattura-elettronica](https://github.com/Slamdunk/php-validatore-fattura-elettronica) | MIT | `tests/*.xml`, nine files |
| `tappoz/` | [tappoz/ffe-fatture-elettroniche](https://github.com/tappoz/ffe-fatture-elettroniche) | LGPL 3.0 or later | `sample_data/*.xml`, `fe-xml-sample/personas/*.xml`, `fe-xml-sample/templates/*.xml`, two hundred forty six files |
| `utenzepa/` | [Comune-di-Montesilvano/UtenzePA](https://github.com/Comune-di-Montesilvano/UtenzePA) | EUPL 1.2 | `*.xml`, `*.p7m`, ten files |
| `validatefin/` | [ValidateFin sample download](https://validatefin.com/en/resources/samples/fatturapa-fpr12-signed-p7m) | CC BY 4.0 | `IT06363391001_0000*.xml`, `IT06363391001_00002.xml.p7m`, four files |
| `zugferd-corpus/` | [ZUGFeRD/corpus](https://github.com/ZUGFeRD/corpus) | Apache 2.0 | `eigor/valid/*.xml`, `official/valid/*.xml` |

All the licences permit redistribution of the sample files. The sixteen new
sources were copied at the commits `45b56827`, `fbd03966`, `d6b91a0b`,
`4e3b62e1`, `ab6bb846`, `c9da86ea`, `ecf8ec20`, `aceda310`, `c3d3f7fe`,
`59e33334`, `3f91becd`, `8eb300f7`, `2a4427de`, `1d70ec55`, `07f86a9c` and
`82e886fb` in the order of the table. The ten sources added here were copied
at the commits `638443f0`, `b6189978`, `18b852fa`, `6d2a9d0e`, `b8b043fc`,
`6bcc8bd0`, `07fd3658`, `3f3b0104` and `bc7a7113` in the order of the table,
and `validatefin/` is a web download from the sample page of the site,
retrieved on 2026-11-07, which the site offers under `CC BY 4.0`.

Suites examined and not copied, with the reason. `AGPL` licences that are
incompatible with redistribution, `italia/fatturapa-testsdi`,
`italia/fatturapa-magento`, `gigidn/...`, `provbz/...`, `Micronaet/...`,
`sidis405/...`, `consiglionazionale/...`, `ktecsrl/PyFePA`, the `AGPL`
`odoo` and `frappe` connectors. No usable licence, `italia/fatturapa-python`,
`s2software/fatturapa`, `link-it/govfat`, `link-it/govway`,
`cfrancescop/openspcoop`, `DelphiClubItalia/...`, `advinser/...`,
`walter-arrighetti/...`, `AgID/EeISI`, `erpreciso/fpa-reader`, `lmillucci/...`,
`rives1/...`, `Bruner4/...`, `orjn/...`, `darkrei08/...`. Byte duplicates of
files already in the corpus or in `spec/samples/`, `azuki-beans/fatturapa`,
`FatturaElettronica/FatturaElettronica.Extensions`, the
`fatture-elettroniche-explorer/duplicato/` directory and the `invoice-extract`
`_copia` file. Templates only,
`Solede-SA/...`. The `simevo` `malicious/` directory is a set of `XML` bombs
rather than invoices and is not copied. `fexplorer/non_fattura.xml` is a
Delphi `TISFunction` document, the `kamerozkan` `UBL` file is not `FatturaPA`,
and `slamdunk/ok_semplificata` is a placeholder comment. Not invoices and not
copied, the `fatture-elettroniche-explorer/ATTESO.json` file, the
`tappoz/fe-xml-sample` `yaml` catalogs, `utenzepa/documento.pdf`,
`p7m-apri/documento.pdf` and `p7m-fattura-decoder/not-base64.txt`.
`odoo-fatturapa-viewer/test.xml.p7m` and `p7m-apri/TEST.p7m` carry the
certificates of real persons and are excluded.

## Outcomes

Four hundred and ninety two files. Three hundred and seventy eight read,
validate and round-trip, including the signed files unwrapped from their
`.p7m` wrapper. One hundred and eight fail the `XSD` validation of `spec/`,
listed below with the reason `xmllint` reports. Six signed files do not unwrap
at all, because their `CMS` structure is damaged or absent, and that is what
their upstream suite exercises them for.

Forty of the round-tripped invoices carry a `Signature` element. Their
`xs:any` content, the filter-2 `XPath` transform and the XAdES object, is
preserved through the round-trip, up to the in-scope namespaces of that
content, which differ when the source prefix differs from the one the writer
picks, an `XML 1.0` limit. The round-trip test therefore checks equality
outside the signature and compares the two signatures with the in-scope
namespaces of the `xs:any` content cleared. The written bytes of the signed
documents are `XSD`-validated like the ones of the unsigned ones.

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
| `estratto/05-non-imponibile.xml` | `EsigibilitaIVA` not expected on the non-taxable line |
| `fatture-elettroniche-explorer/carburante/IT00000000001_00004.xml` | `Numero` carries an `onmouseover` XSS payload that fails the pattern |
| `fatture-elettroniche-explorer/casi-limite/IT02345678900_MT_00001.xml` | `MetadatiInvioFile` is an SDI message root, no global element in the invoice schema |
| `fatture-elettroniche-explorer/casi-limite/documento-non-fattura.xml` | `Documento` root, not a `FatturaPA` document |
| `fatture-elettroniche-explorer/casi-limite/namespace-ns2.xml`, `casi-limite/namespace-senza-prefisso.xml` | `v1.2` default namespace, `xmllint` reports `FatturaElettronicaHeader` as not expected, two files |
| `fatture-elettroniche-explorer/casi-limite/senza-data.xml` | `Numero` where `Data` is expected |
| `fatture-elettroniche-explorer/casi-limite/xml-non-valido.xml` | prefix `p` on `FatturaElettronica` not defined |
| `invoice-extract/IT01234567897_00001_RC_001.xml` | `RicevutaConsegna` is an SDI message root, no global element in the invoice schema |
| `invoice-extract/IT01234567897_00001.xml`, `IT02345678904_00042.xml`, `IT03456789019_0A1B2.xml`, `IT05678901231_00003.xml`, `IT04567890126_00017.xml.p7m` | `RegimeFiscale` in a position the v1.2 sequence does not expect, five files |
| `invoice-extract/IT03456789019_0A1B3.xml` | `v1.2` default namespace, `xmllint` reports `FatturaElettronicaHeader` as not expected |
| `odoo-fatturapa-viewer/test-bad.xml` | `UnclosedTag` on line 34 leaves `FatturaElettronica` open |
| `p7m-apri/documento-esempio.pdf.p7m`, `documento-esempio.pdf.p7m.p7m` | `PDF` content, not `XML`, two files |
| `p7m-apri/fattura-firmata-esempio.xml.p7m` | the unwrapped invoice carries `ContattiTrasmittente`, not expected |
| `tappoz/fe-xml-sample/templates/tmpl_*.xml` | `IdPaese` is the `{{CEDENTE_ID_PAESE}}` placeholder, `{{SUPPLIER_ID_PAESE}}` in `tmpl_ricevuta_td01`, eight files |
| `tappoz/sample_data/IT01234567890_GCE001_fail_BR_GCE001.xml`, `IT12345678901_ddddd_SDI_PEC_fail_BR_00330_CHECKME_.xml` | `PECDestinatario` is the `[EMAIL]` placeholder, fails the pattern, two files |
| `tappoz/sample_data/IT12345678901_ddddd_DATAFATTURACOLLEGATA_ANTECEDENTE_fail_BR_00418.xml` | `Data` in the linked document is not expected |
| `tappoz/sample_data/IT12345678901_ddddd_NATURA_N6_SCISSIONE_fail_BR_00420.xml` | `EsigibilitaIVA` not expected in the line sequence |
| `tappoz/sample_data/recursive_test/invoice_cost_1.xml`, `recursive_test/subdir/invoice_cost_2.xml` | `DatiAnagrafici` missing child elements, two files |
| `utenzepa/dt.xml`, `utenzepa/se.xml` | `v1.0` SDI messages missing the child elements the message schema expects, two files |
| `utenzepa/mt.xml` | `FileMetadati` has no matching global declaration in the message schema |
| `utenzepa/fattura.xml`, `utenzepa/fattura-aca.xml`, `utenzepa/fattura-pod.xml` | `DatiAnagrafici` missing child elements, three files |
| `utenzepa/fattura-firmata.xml.p7m`, `utenzepa/fattura-firmata-b64.xml.p7m` | the unwrapped invoices are missing child elements in `DatiAnagrafici`, two files |

The three files that fail with `FatturaElettronicaHeader` not expected use the
`v1.2` namespace of the schema as a default or prefixed namespace, and
`xmllint` reports the first body element as unexpected there: a `libxml`
namespace-handling quirk, not a defect of the document or the schema.

### Signed files that do not unwrap

| File | State |
| --- | --- |
| `fatturaelettronica-net/IT02182030391_31_tampered.Base64.xml.p7m` | outer `SEQUENCE` length damaged |
| `fatturaelettronica-net/IT02182030391_31_tampered.xml.p7m` | nested `SEQUENCE` length damaged |
| `fatturaelettronica-php/invalid.xml.p7m` | the trailing `00 00` end of contents is `00 01` |
| `fatture-elettroniche-explorer/firmate/IT02345678900_00012.xml.p7m` | `PEM` text, outside the raw `DER` or base64 input contract |
| `odoo-fatturapa-viewer/test-bad.xml.p7m` | a placeholder sentence, not a `CMS` structure |
| `p7m-fattura-decoder/truncated.p7m` | a `DER` `SignedData` truncated mid-structure |

The remaining `FatturaElettronica.NET` signed files exercise the `BER`
indefinite length form that the .NET runtime writes, and the signer that
splits the `eContent` into 1000 byte chunks under a constructed `OCTET
STRING`. `fatturaelettronica-php/IT00484960588_ERKHK.xml.p7m` is a signed
invoice with a 1.4 MB attachment.

### Files on which `validate` reports violations

Every reported violation is a true SDI finding about the file, checked against
the Elenco Controlli v2.0 text, and against the Allegato A 1.9.1 operative
specification, which refines the 00313, 00330, 00401, 00430, 00471 and 00472
criteria of the version 2.0 list. Every other readable invoice is clean.

| File | Codes | What the file does |
| --- | --- | --- |
| `fatturaelettronica-php/IT01234567899_000sq.xml` | 00421 twice, 00445 twice | a riepilogo `Imposta` off by exactly one cent, and the generic nature `N2` on a line and a riepilogo |
| `fatturaelettronica-php/semplificata/ESEMPIO NDC SEMPL.xml` | 00418 | a simplified `TD08` with a `Data` preceding the linked `DataFR` |
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
| `estratto/08-imposta-errata.xml` | 00421 | riepilogo `Imposta` off from the recomputed value |
| `estratto/09-zero-senza-natura.xml` | 00400, 00429 | a zero-rate line and a riepilogo missing `Natura` |
| `odoo-fatturapa-viewer/test.xml` | 00471 | a `TD01` with a seller and a buyer sharing the same `IdFiscaleIVA` |
| `tappoz/sample_data/IT01234567890_GCE009_fail_BR_00473_GCE009.xml` | 00473 | seller `IdPaese` not allowed for the document type |
| `tappoz/sample_data/IT12345678901_ddddd_ALIQUOTA_FRACTION_fail_BR_00424.xml` | 00424 three times | a rate not expressed as a percentage on two lines and the riepilogo |
| `tappoz/sample_data/IT12345678901_ddddd_ALIQUOTA_MISSING_RIEPILOGO_fail_BR_00443.xml` | 00419, 00443, 00422 | a line rate missing from `DatiRiepilogo`, plus a riepilogo `ImponibileImporto` off |
| `tappoz/sample_data/IT12345678901_ddddd_BOTH_FOREIGN_fail_BR_00476.xml` | 00476 | seller and buyer `IdPaese` both different from `IT` |
| `tappoz/sample_data/IT12345678901_ddddd_CASSA_ALIQ0_NO_NATURA_fail_BR_00413.xml` | 00413 | a cassa `Natura` missing at a zero `AliquotaIVA` |
| `tappoz/sample_data/IT12345678901_ddddd_CASSA_NONZERO_WITH_NATURA_fail_BR_00414.xml` | 00414, 00444, 00422 | a cassa `Natura` present at a non-zero rate and missing from the riepilogo, plus a riepilogo `ImponibileImporto` off |
| `tappoz/sample_data/IT12345678901_ddddd_CASSA_NONZERO_WITH_NATURA_fail_BR_00414_00444_GCE004.xml` | 00414, 00444 | a cassa `Natura` present at a non-zero rate and missing from the riepilogo |
| `tappoz/sample_data/IT12345678901_ddddd_CASSA_RITENUTA_NO_DATIRIT_fail_BR_00415.xml` | 00415 | `DatiRitenuta` missing for a cassa with `Ritenuta SI` |
| `tappoz/sample_data/IT12345678901_ddddd_CC_NO_IDS_fail_BR_00417.xml` | 00417 | buyer `IdFiscaleIVA` and `CodiceFiscale` both absent |
| `tappoz/sample_data/IT12345678901_ddddd_CEDENTE_EQ_CESSIONARIO_fail_BR_00471.xml` | 00471 | seller and buyer identical for the document type |
| `tappoz/sample_data/IT12345678901_ddddd_CEDENTE_NEQ_CESSIONARIO_TD21_fail_BR_00472.xml` | 00472 | seller and buyer not identical for `TD21` |
| `tappoz/sample_data/IT12345678901_ddddd_CODICE_DEST_6CHARS_fail_BR_00427.xml` | 00427 | `CodiceDestinatario` length wrong for the `FormatoTrasmissione` |
| `tappoz/sample_data/IT12345678901_ddddd_DUPL_LOTTO_fail_BR_00409.xml` | 00409 | a duplicate invoice in the same document |
| `tappoz/sample_data/IT12345678901_ddddd_IMPONIBILE_WRONG_fail_BR_00422.xml` | 00422 | a riepilogo `ImponibileImporto` off from the recomputed value |
| `tappoz/sample_data/IT12345678901_ddddd_IMPOSTA_WRONG_fail_BR_00421.xml` | 00421 | a riepilogo `Imposta` off from the recomputed value |
| `tappoz/sample_data/IT12345678901_ddddd_IMPOSTA_WRONG_fail_BR_00421_CHECKME_.xml` | 00421 | a riepilogo `Imposta` off from the recomputed value |
| `tappoz/sample_data/IT12345678901_ddddd_INVCONT_extra_UE_fail_BR_00313.xml` | 00313 | the `XXXXXXX` `CodiceDestinatario` for a buyer established in `IT` |
| `tappoz/sample_data/IT12345678901_ddddd_IT_CEDENTE_TD17_fail_BR_00473_GCE009.xml` | 00473 | seller `IdPaese` not allowed for the document type |
| `tappoz/sample_data/IT12345678901_ddddd_NATURA_GENERIC_N2_fail_BR_00445.xml` | 00445 twice | the generic nature `N2` on a line and a riepilogo |
| `tappoz/sample_data/IT12345678901_ddddd_NATURA_MISMATCH_RIEPILOGO_fail_BR_00444.xml` | 00444 | a line `Natura` missing from the `DatiRiepilogo` blocks |
| `tappoz/sample_data/IT12345678901_ddddd_NUMERO_NO_DIGITS_fail_BR_00425.xml` | 00425 | `Numero` without any digit |
| `tappoz/sample_data/IT12345678901_ddddd_PREZZO_TOTALE_WRONG_fail_BR_00423.xml` | 00423 | a line `PrezzoTotale` off from the recomputed value |
| `tappoz/sample_data/IT12345678901_ddddd_RIEPILOGO_NONZERO_WITH_NATURA_fail_BR_00401_00430.xml` | 00430 | a riepilogo `Natura` present at a non-zero `AliquotaIVA` |
| `tappoz/sample_data/IT12345678901_ddddd_RIEPILOGO_ZERO_NO_NATURA_fail_BR_00400_00429.xml` | 00400 twice, 00429 | two zero-rate lines and a riepilogo missing `Natura` |
| `tappoz/sample_data/IT12345678901_ddddd_RITENUTA_MISSING_fail_BR_00411.xml` | 00411 | `DatiRitenuta` missing for a line with `Ritenuta SI` |
| `tappoz/sample_data/IT12345678901_ddddd_SCONTO_DOC_NO_VALUE_fail_BR_00437.xml` | 00437 | a document `ScontoMaggiorazione` with `Tipo` and neither value |
| `tappoz/sample_data/IT12345678901_ddddd_SCONTO_LINE_NO_VALUE_fail_BR_00438.xml` | 00438 | a line `ScontoMaggiorazione` with `Tipo` and neither value |
| `tappoz/sample_data/IT12345678901_ddddd_SDI_PEC_fail_BR_00330.xml` | 00330 | the `PECDestinatario` is the SdI operational mailbox `sdi01@pec.fatturapa.it` |
| `tappoz/sample_data/IT12345678901_ddddd_TD17_NO_IDIVA_CC_fail_BR_00475.xml` | 00475 | `TD17` without the buyer `IdFiscaleIVA` |
| `tappoz/sample_data/IT12345678901_ddddd_TD21_ZERO_ALIQUOTA_fail_BR_00474.xml` | 00400, 00474 | `TD21` with a zero-rate line missing `Natura` |
| `validatefin/IT06363391001_00003.xml` | 00445 twice, 00421, 00422 | the generic nature on a line and a riepilogo, plus riepilogo `Imposta` and `ImponibileImporto` off |
| `validatefin/IT06363391001_00004.xml` | 00427, 00425 | `CodiceDestinatario` length wrong and `Numero` without any digit |
