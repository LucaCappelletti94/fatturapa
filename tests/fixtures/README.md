# Test fixtures

The `.key` files are the private keys of a throwaway test PKI that exists only to sign these fixtures. They protect nothing and are published on purpose. `generate.sh` recreates the whole PKI and every signed `.p7m` from the official sample invoice.

`TSL-IT.xml` is AgID's Italian trusted list, downloaded from `https://eidas.agid.gov.it/TL/TSL-IT.xml`. `tsl-synthetic.xml` is a minimal hand-written list for the service status and type rules.

The hand-written invoices in `invoices/` are built against the specification with fictitious parties only. Every `partita IVA` and foreign identifier is a synthetic check-valid number, never a real company's number. The foreign seats carry the `CAP` `00000` and the foreign postal code inside the `Indirizzo`, the form the Agenzia FAQ prescribes.
