# pint-sg-validator

A fast, dependency-light validator for **Peppol PINT SG** — the e-invoice
format Singapore businesses exchange over the InvoiceNow network. Written in
Rust; usable as a library or a single-binary CLI.

```
$ pintsg invoice.xml
✓ conforms — no findings
PASS — 0 error(s), 0 warning(s)

$ pintsg broken.xml
ERROR   [PINT-SG-01] CustomizationID is 'urn:peppol:pint:billing-1@aus-1', expected 'urn:peppol:pint:billing-1@sg-1' for PINT SG billing
ERROR   [EN16931-CO-15] TaxInclusiveAmount (119.00) != TaxExclusiveAmount (100.00) + TaxTotal (9.00) = 109.00
FAIL — 2 error(s), 0 warning(s)
```

Exit code is `0` when the document conforms and `1` when it does not, so it
drops into a CI pipeline or a pre-send gate unchanged. `--json` emits a
machine-readable report.

## Why this exists

Singapore's GST InvoiceNow requirement is phasing in from 2026, and every
GST-registered business will have to emit invoices that conform to PINT SG.
Most errors are dull and repeatable: the wrong customization identifier, a
missing participant scheme, totals that do not add up, GST coded as if it were
European VAT. This catches those before a document reaches an Access Point and
bounces.

It exists as open source because a shared, readable reference implementation is
better for the whole Singapore Peppol community than everyone re-deriving the
same rules privately.

## What it checks (v0.1)

| Area | Examples |
|---|---|
| PINT SG profile | CustomizationID `urn:peppol:pint:billing-1@sg-1`, ProfileID `urn:peppol:bis:billing` |
| Identity | Supplier/customer EndpointID present; Singapore UEN scheme `0195` |
| GST (not VAT) | Valid tax categories; supplier GST registration when standard-rated; current-rate sanity |
| Arithmetic (EN 16931) | tax-inclusive = tax-exclusive + tax; line total = sum of lines |
| Structure | mandatory business terms, ISO date, ISO 4217 currency, at least one line |

Findings are graded **error** (would not conform), **warning** (conforms but
likely wrong), and **info** (worth knowing).

### Scope and honesty

This is **not** a drop-in replacement for the official PINT SG Schematron. It
implements the high-value subset by hand. A clean result here means "very
likely to pass an Access Point", not "guaranteed". Running the published `.sch`
rule sets in addition is on the roadmap. Rule identifiers and values that can
change with a spec revision (the customization id, the GST standard rate) are
constants at the top of `src/rules.rs`, each citing its source, so corrections
are a one-line change anyone can verify against the live specification.

## Library use

```rust
use pint_sg_validator::validate;

let report = validate(&xml_string);
if !report.conforms {
    for f in &report.findings {
        eprintln!("[{}] {}", f.rule, f.message);
    }
}
```

## Install / build

```bash
cargo build --release   # binary at target/release/pintsg
cargo test              # the rule suite
```

## Contributing

Corrections to rules against the published specification are especially
welcome — cite the spec section. Please read `NOTICE.md` first: this is a
clean-room implementation from public specifications and must stay that way, so
do not paste code from other invoicing projects.

## License

[Functional Source License 1.1 (MIT future grant)](LICENSE.md). Use it freely
for anything that is not a competing e-invoice-validation product; each release
converts to the MIT License two years after publication. See `NOTICE.md` for
provenance.

Not affiliated with or endorsed by OpenPeppol AISBL or IMDA.
