# Test fixtures

The `.key` files are the private keys of a throwaway test PKI that exists only to sign these fixtures. They protect nothing and are published on purpose. `generate.sh` recreates the whole PKI and every signed `.p7m` from the official sample invoice.

`TSL-IT.xml` is AgID's Italian trusted list, downloaded from `https://eidas.agid.gov.it/TL/TSL-IT.xml`. `tsl-synthetic.xml` is a minimal hand-written list for the service status and type rules.
