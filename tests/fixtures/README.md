# Test fixtures

The `.key` files are the private keys of a throwaway test PKI that exists only to sign these fixtures. They protect nothing and are published on purpose. `generate.sh` recreates the whole PKI and every signed `.p7m` from the official sample invoice.

`TSL-IT.xml` is AgID's Italian trusted list, downloaded from `https://eidas.agid.gov.it/TL/TSL-IT.xml`. `tsl-synthetic.xml` is a minimal hand-written list for the service status and type rules.

The hand-written invoices in `invoices/` are built against the specification with fictitious parties only. Every `partita IVA` and foreign identifier is a synthetic check-valid number, never a real company's number. The foreign seats carry the `CAP` `00000` and the foreign postal code inside the `Indirizzo`, the form the Agenzia FAQ prescribes.

The `TD01`, `TD02`, `TD03`, `TD05`, `TD09`, `TD25` and `TD26` files below pin one fixture per ordinary document type the corpus lacked. Each is anonymized from a public source or written by hand against the *Guida alla compilazione* 1.10, and each is covered by a test in `document_types.rs`. The anonymized parties reuse the crate's synthetic identifiers.

| File | Type | Source | Rules it pins |
| --- | --- | --- | --- |
| `td02-advance.xml` | `TD02` | the `TD02` advance in `InventatoIVA2021.txt`, [BananaAccounting/Italia](https://github.com/BananaAccounting/Italia/blob/cb5c281296eaf42f521aa17b5fac944bc25bb8fd/FattureElettroniche/B2B/test/testexpected/ch.banana.it.efattura.b2b.test/test1/InventatoIVA2021.txt) | the down payment links no document and names two different parties |
| `td01-advance-balance.xml` | `TD01` | hand-written from *Guida* p. 7, the balance of the Banana advance | the negative deduction line and the `DatiFattureCollegate` link to the advance |
| `td03-advance-parcella.xml` | `TD03` | hand-written from *Guida* p. 7, the shared `TD02`/`TD03` section | the down payment for professional fees, the sole-trader `CodiceFiscale` |
| `td05-debit-note.xml` | `TD05` | `it_edi_debit_note.xml`, [odoo/odoo](https://github.com/odoo/odoo/blob/25cadfbb173b7d1be350dffbacf6ea66109f5d9d/addons/l10n_it_edi/tests/export_xmls/it_edi_debit_note.xml) | the debit note links its invoice. The source link misses the `IdDocumento`, which the fixture adds, and the `MP05`/`BIC` payment becomes `MP02` with the synthetic `IBAN` |
| `td09-simplified-debit-note.xml` | `TD09` | the simplified `TD07` example in [forum.italia.it 28826](https://forum.italia.it/t/fattura-semplificata-td07-esempio-xml/28826), the only complete `FSM10` example found online | the `FSM10` form of the debit note, with the `DatiFatturaRettificata` link |
| `td25-triangolazione.xml` | `TD25` | hand-written from *Guida* p. 27 | the deferred invoice of the triangular transaction, issued by the seller with no linked document |
| `td26-amortizable.xml` | `TD26` | hand-written from *Guida* p. 28 | the sale of an amortizable good that does not count toward the turnover |
