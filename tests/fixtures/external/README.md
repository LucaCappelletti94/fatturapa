# External signature and trusted list material

Third-party material for the `signed` and `trust_list` modules. Every file is
copied byte-identical from a recorded upstream commit, and the observed
outcome of `unwrap` and `verify` is pinned in `tests/signed.rs` while the
`trust_list` anchor counts are pinned in `tests/trust_list.rs`. The
`include` list in `Cargo.toml` keeps this directory out of the published
crate.

## Sources

| Directory | Upstream | Licence | Commit or snapshot | Copied |
|---|---|---|---|---|
| `dss/` | https://github.com/esig/dss | LGPL-2.1, the `dss/LICENSE` file copied alongside | `9f3af8f2ca672b8948e0a3ee8687e53adf9f5c33` | 82 signature files plus `LICENSE` |
| `tsl-it/` | https://eidas.agid.gov.it/TL/TSL-IT.xml through the `Wayback Machine` | CC BY 4.0, https://www.agid.gov.it/it/note-legali | Wayback raw snapshots, one per year | 6 snapshots |
| `lotl/` | https://ec.europa.eu/tools/lotl/eu-lotl.xml | CC BY 4.0, https://ec.europa.eu/info/legal-notice_en | one fetch | 1 file |

### `dss/`

From the EU Digital Signature Services test resources, the signature files
only. No duplicate of this set exists in `tests/corpus`, `spec/samples` or
the rest of `tests/fixtures`.

- 77 files from `dss-cades/src/test/resources/validation/`, 32 at the top
  level and 45 in the `dss-1220/`, `dss-1344/`, `dss-1419/`, `dss-1469/`,
  `dss-1670/`, `dss-2011/`, `dss-916/` and `evidence-record/` subdirectories
- `dss/cms/timestamp.tst` from `dss-cms/src/test/resources/timestamp.tst`
- `dss/spi/archive_timestamp.tst`, `dss/spi/disig.tst` and
  `dss/spi/tst_with_broken_ocsp.tst` from `dss-spi/src/test/resources/`
- `dss/validation/tst-two-refs.tst` from `dss-validation/src/test/resources/`

### `tsl-it/`

Raw `id_` Wayback snapshots of the Italian trusted services list, one per
year from 2017 to 2024. The `20190824181858` snapshot is truncated at exactly
1 MiB, a negative case with an unclosed root element. The lists are the
public qualified CA list of the Republic of Italy and carry the VAT
identifiers (`VATIT-...`) of real companies. Those are public company
identifiers, not personal data of private persons.

### `lotl/`

The EU List of Trusted Lists. The file carries no licence statement. The
European Commission legal notice states that content owned by the EU on its
website is licensed under the Creative Commons Attribution 4.0 International
(CC BY 4.0) licence. The https://ec.europa.eu/tools/lotl page redirects to
the digital building blocks page, which cites eIDAS Article 22(4) and
Commission Decision C(2015) 1505.

## Pinned outcomes

- The 82 signature files pin in `external_unwrap_outcomes_are_pinned` and
  `external_verify_outcomes_are_pinned` of `tests/signed.rs`. Four of them
  verify with their embedded certificates as the anchors, each with a chain
  of length two. The `.tst` tokens unwrap to
  `UnsupportedContentType(1.2.840.113549.1.9.16.1.4)`, the `TSTInfo`
  content type, because the crate is CMS `id-data` only.
- The historical `tsl-it/` snapshots pin in
  `historical_snapshots_parse_to_the_recorded_anchors` with 113 anchors
  (2017), 145 (2021), 143 (2022), 143 (2023) and 138 (2024). The truncated
  2019 snapshot fails with `TrustListError::NotTrustedList`.
- The `lotl/` list pins in `eu_lotl_parses_to_zero_anchors`. It lists
  trusted lists, not qualified certificate authorities, and must parse to
  zero anchors without an error.

Two LTA files deserve a note. `cades-lta-en319122.cms` and
`cades-lta-ts101733.cms` carry a base signature that does not verify under
the embedded signer certificate named by the `SignerInfo`. The negative
result is confirmed by `openssl cms -verify -noverify`, the Python
`cryptography` package and the crate itself. The DSS test suite asserts a
valid base signature for these files, which it presumably obtains through
its test PKI certificate source rather than the embedded certificate. The
crate verifies the embedded certificate the `SignerInfo` names, so the
pinned outcome is `SignatureInvalid`.

## Rejected files

38 DSS files carry personal data of a real person, a name, a codice fiscale,
a document number or an e-mail address in a certificate or in the signed
content, and were not copied. The list names the files only.

- `Signature-C-HU_MIC-1.p7m`, `Signature-C-CZ_SIX-1.p7m`,
  `Signature-C-DE_CRY-3.p7m`, `Signature-C-DE_CRY-4.p7m`,
  `Signature-C-A-XL-1.p7m`, `Signature-C-BES-4.p7m`, `Signature-CBp-B-1.p7m`,
  `Signature-C-X-1.p7m`, `Signature-C-B-B-8.p7m`, `Signature-C-B-LTA-10.p7m`,
  `Signature-C-HU_POL-3.p7m`, `Signature-CBp-LT-2.p7m` and
  `Signature-15-CBp-LT-2.p7m`
- `cades-invalid-ocsp-responses-encoding.p7s`, `cades-double-signing-time.p7m`,
  `cades-double-signing-certificate.p7m`, `cades-duplicated-signed-attrs.p7m`,
  `cades-e-lt.p7m`, `cades-ats-v3-rev-val-ocsp.p7s`,
  `cades-ats-v3-wrong-ocsp.p7s` and `dss2059.p7s`
- `dss-916/test.txt.signed.qes.attached.p7s`,
  `dss-916/test.txt.signed.qes.detached.p7s`, `dss-768/FD1&FD2&FEA.pdf.p7m`,
  `dss-951/NexU-CAdES-B-B-Detached-Sha512.p7m`, `dss-1401/sig_with_atsv2.p7s`,
  `dss-1619/CAdES-XL-T1-Double-AV2.png.p7m`,
  `dss-1892/detached_with_counter_sig.p7m`,
  `dss-667/BER_reordered_prova.txt.p7m` and
  `dss-667/DER_reordered_prova.txt.p7m`
- `evidence-record/C-B-B-basic.p7m`, `evidence-record/C-E-ERS-basic.p7m`,
  `evidence-record/C-B-B-detached.p7s`, `evidence-record/Double-C-B-B-basic.p7m`
  and `evidence-record/Double-C-E-ERS-basic.p7m`
- `dss-cms/src/test/resources/cades.p7m`,
  `dss-cookbook/src/test/resources/signature-pool/signedCadesB.p7m` and
  `signedFile.pdf.p7s`

All remaining DSS test resources were scanned and were either copied or are
not signature tokens.
