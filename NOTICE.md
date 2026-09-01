# Provenance and licensing notice

`pint-sg-validator` is an **independent, clean-room implementation** of the
validation rules for Peppol PINT SG billing documents.

Its rules are derived solely from public specifications and standards:

- The Peppol PINT SG billing specification, published by OpenPeppol AISBL
  (https://docs.peppol.eu/poac/sg/pint-sg/).
- EN 16931 (the European semantic invoice standard PINT builds upon).
- The OASIS UBL 2.1 schemas.
- Singapore participant-identifier and GST facts published by IMDA and IRAS.

Specifications, standards, identifier strings, code lists, and the arithmetic
of an invoice are facts and public requirements, not authored code. This
project was written from those documents.

**It is not derived from, and does not incorporate, source code from any other
invoicing implementation** — in particular none from any GPL-, LGPL-, or
AGPL-licensed project. No third-party source was copied or translated. This
clean-room provenance is what allows the project to be offered under the
license in `LICENSE.md`.

If you believe any part of this code reproduces copyrighted expression from
another project rather than implementing a public specification, please open an
issue; we will investigate and remove it.
