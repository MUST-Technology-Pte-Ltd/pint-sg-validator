//! # pint-sg-validator
//!
//! A validator for Peppol **PINT SG** (Singapore) billing invoices and credit
//! notes — the format Singapore businesses must exchange over the InvoiceNow
//! (Peppol) network.
//!
//! ## What it does
//!
//! Parse a UBL 2.1 Invoice or CreditNote and check it against the rules that
//! most often go wrong in practice: the PINT SG specification identifiers, the
//! Singapore participant scheme, GST tax coding (Singapore uses GST, not VAT),
//! the document-total arithmetic from EN 16931, and the mandatory business
//! terms. Each finding carries a rule id, a severity, and a human message.
//!
//! ## What it is not (yet)
//!
//! It is not a drop-in replacement for the official Schematron. It implements
//! the high-value subset by hand; the roadmap is to additionally run the
//! published `.sch` rule sets. Treat a clean result here as "very likely to
//! pass an Access Point", not "guaranteed".
//!
//! ## Provenance
//!
//! Implemented clean-room from the public PINT SG specification
//! (<https://docs.peppol.eu/poac/sg/pint-sg/>) and EN 16931. It is **not**
//! derived from, and does not copy, any GPL/LGPL invoicing implementation. See
//! `NOTICE.md`.

mod model;
mod rules;

pub use model::{DocKind, Document};
pub use rules::{Finding, Report, Severity};

mod parse;

/// Validate a UBL billing document supplied as an XML string.
///
/// Returns a [`Report`] even when the XML is malformed — a parse failure is
/// itself reported as a single fatal finding, so a caller never has to handle
/// two different error channels.
pub fn validate(xml: &str) -> Report {
    match parse::parse(xml) {
        Ok(doc) => rules::run(&doc),
        Err(msg) => Report::fatal("XML-PARSE", &msg),
    }
}
