//! The rule set.
//!
//! Every rule is a small function that inspects the parsed [`Document`] and
//! pushes zero or more [`Finding`]s. Rule ids are namespaced by origin so a
//! reader knows where the requirement comes from:
//!
//! * `PINT-SG-*` — Singapore country rules (this crate's reading of the spec)
//! * `EN16931-*` — the European semantic core PINT builds on
//! * `UBL-*`     — structural / cardinality
//!
//! The values that could change with a spec revision (the customization id,
//! the current GST standard rate) are constants at the top of this file, each
//! citing its source, so a correction is a one-line edit a reviewer can verify
//! against the live specification.

use crate::model::*;
use serde::Serialize;

/// PINT SG billing customization identifier.
/// Source: https://docs.peppol.eu/poac/sg/pint-sg/ (Billing v1.x).
/// The `@sg-1` jurisdiction suffix is what distinguishes SG from the base PINT
/// and from other jurisdictions (e.g. `@aus-1`, `@nz-1`).
const PINT_SG_CUSTOMIZATION: &str = "urn:peppol:pint:billing-1@sg-1";

/// Peppol billing process identifier (shared across PINT jurisdictions).
const PEPPOL_BILLING_PROFILE: &str = "urn:peppol:bis:billing";

/// Singapore participant scheme: UEN under ISO/IEC 6523 code `0195`.
/// Source: Peppol Policy for use of Identifiers / SG participant registration.
const SG_UEN_SCHEME: &str = "0195";

/// Current Singapore GST standard rate, in percent. Informational only — an
/// `S`-category line at a different rate is flagged as INFO, not an error,
/// because the rate is set by IRAS and has changed (7% -> 8% -> 9%).
const GST_STANDARD_RATE: f64 = 9.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Would be rejected by an Access Point / does not conform.
    Error,
    /// Conforms, but likely wrong or risky — review before sending.
    Warning,
    /// Worth knowing; no action required.
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// True when there are no `Error` findings.
    pub conforms: bool,
    pub errors: usize,
    pub warnings: usize,
}

impl Report {
    fn from(findings: Vec<Finding>) -> Self {
        let errors = findings.iter().filter(|f| f.severity == Severity::Error).count();
        let warnings = findings.iter().filter(|f| f.severity == Severity::Warning).count();
        Report { conforms: errors == 0, errors, warnings, findings }
    }

    /// A single fatal finding (used when the XML will not even parse).
    pub fn fatal(rule: &str, message: &str) -> Self {
        Report::from(vec![Finding {
            rule: rule.into(),
            severity: Severity::Error,
            message: message.into(),
        }])
    }
}

struct Ctx {
    out: Vec<Finding>,
}

impl Ctx {
    fn err(&mut self, rule: &str, msg: impl Into<String>) {
        self.out.push(Finding { rule: rule.into(), severity: Severity::Error, message: msg.into() });
    }
    fn warn(&mut self, rule: &str, msg: impl Into<String>) {
        self.out.push(Finding { rule: rule.into(), severity: Severity::Warning, message: msg.into() });
    }
    fn info(&mut self, rule: &str, msg: impl Into<String>) {
        self.out.push(Finding { rule: rule.into(), severity: Severity::Info, message: msg.into() });
    }
    /// Require a field; emit an error naming the business term if absent.
    fn require(&mut self, rule: &str, bt: &str, value: &Option<String>) {
        if value.as_deref().unwrap_or("").is_empty() {
            self.err(rule, format!("{bt} is mandatory but missing or empty"));
        }
    }
}

/// Parse an amount, tolerating thousands separators the way senders emit them.
fn amount(s: &Option<String>) -> Option<f64> {
    s.as_deref()?.replace(',', "").trim().parse::<f64>().ok()
}

/// Two monetary values equal within half a cent — invoices round to 2 dp, and
/// exact float equality would false-fail on totals summed a different way.
fn money_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.005
}

pub fn run(doc: &Document) -> Report {
    let mut c = Ctx { out: Vec::new() };

    profile(&mut c, doc);
    header(&mut c, doc);
    parties(&mut c, doc);
    tax(&mut c, doc);
    totals(&mut c, doc);
    lines(&mut c, doc);

    Report::from(c.out)
}

fn profile(c: &mut Ctx, doc: &Document) {
    match doc.customization_id.as_deref() {
        None => c.err("PINT-SG-01", "CustomizationID (BT-24) is missing"),
        Some(v) if v == PINT_SG_CUSTOMIZATION => {}
        Some(v) => c.err(
            "PINT-SG-01",
            format!(
                "CustomizationID is '{v}', expected '{PINT_SG_CUSTOMIZATION}' for PINT SG billing"
            ),
        ),
    }
    match doc.profile_id.as_deref() {
        None => c.err("PINT-SG-02", "ProfileID (BT-23) is missing"),
        Some(v) if v == PEPPOL_BILLING_PROFILE => {}
        Some(v) => c.err(
            "PINT-SG-02",
            format!("ProfileID is '{v}', expected '{PEPPOL_BILLING_PROFILE}'"),
        ),
    }
}

fn header(c: &mut Ctx, doc: &Document) {
    c.require("UBL-01", "Invoice number (BT-1)", &doc.id);
    c.require("UBL-02", "Issue date (BT-2)", &doc.issue_date);
    if let Some(d) = &doc.issue_date {
        // PINT dates are xs:date: YYYY-MM-DD.
        let ok = d.len() == 10
            && d.as_bytes()[4] == b'-'
            && d.as_bytes()[7] == b'-'
            && d.bytes().enumerate().all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit());
        if !ok {
            c.err("EN16931-02", format!("Issue date '{d}' is not in YYYY-MM-DD form"));
        }
    }
    c.require("UBL-03", "Document type code", &doc.type_code);
    match doc.currency.as_deref() {
        None => c.err("UBL-04", "DocumentCurrencyCode (BT-5) is missing"),
        Some(cur) if cur.len() != 3 || !cur.bytes().all(|b| b.is_ascii_uppercase()) => {
            c.err("EN16931-05", format!("Currency '{cur}' is not a 3-letter ISO 4217 code"));
        }
        Some(cur) if cur != "SGD" => {
            c.info("PINT-SG-05", format!("Currency is '{cur}'; SGD is usual for domestic SG invoices"));
        }
        _ => {}
    }
}

fn check_party(c: &mut Ctx, who: &str, rule_base: &str, p: &Party) {
    if p.name.as_deref().unwrap_or("").is_empty() {
        c.err(&format!("{rule_base}-NAME"), format!("{who} name is missing"));
    }
    match p.endpoint.value.as_deref() {
        None | Some("") => c.err(
            &format!("{rule_base}-EP"),
            format!("{who} EndpointID is missing — a party cannot be addressed on the network without it"),
        ),
        Some(_) => match p.endpoint.scheme.as_deref() {
            None => c.err(
                &format!("{rule_base}-EPSCHEME"),
                format!("{who} EndpointID has no schemeID"),
            ),
            Some(SG_UEN_SCHEME) => {} // SG UEN — expected
            Some(other) => c.info(
                &format!("{rule_base}-EPSCHEME"),
                format!("{who} EndpointID scheme is '{other}', not the Singapore UEN scheme '{SG_UEN_SCHEME}'"),
            ),
        },
    }
}

fn parties(c: &mut Ctx, doc: &Document) {
    check_party(c, "Supplier", "PINT-SG-SELLER", &doc.supplier);
    check_party(c, "Customer", "PINT-SG-BUYER", &doc.customer);

    // If any tax is charged, the seller must carry a GST registration id.
    let charges_tax = doc.tax_subtotals.iter().any(|t| t.category.as_deref() == Some("S"));
    if charges_tax && doc.supplier.tax_company_id.as_deref().unwrap_or("").is_empty() {
        c.err(
            "PINT-SG-SELLER-GST",
            "Standard-rated GST is charged but the supplier has no GST registration id (PartyTaxScheme/CompanyID)",
        );
    }
}

fn tax(c: &mut Ctx, doc: &Document) {
    if doc.tax_total_amount.is_none() {
        c.err("EN16931-TAX", "TaxTotal/TaxAmount (BT-110) is missing");
    }
    if doc.tax_subtotals.is_empty() {
        c.err("EN16931-TAXCAT", "No tax subtotal / category found");
        return;
    }
    // GST category codes per EN 16931 UNCL5305 as used by PINT SG.
    const VALID: &[&str] = &["S", "Z", "E", "G", "O", "AE"];
    for st in &doc.tax_subtotals {
        match st.category.as_deref() {
            None => c.err("EN16931-TAXCAT", "A tax subtotal has no category code"),
            Some(code) if !VALID.contains(&code) => {
                c.warn("PINT-SG-TAXCAT", format!("Tax category '{code}' is unusual for SG GST (expected one of {VALID:?})"));
            }
            Some("S") => {
                match st.percent.as_deref().and_then(|p| p.parse::<f64>().ok()) {
                    None => c.err("EN16931-TAXRATE", "Standard-rated (S) subtotal has no percent"),
                    Some(r) if (r - GST_STANDARD_RATE).abs() > f64::EPSILON => c.info(
                        "PINT-SG-GSTRATE",
                        format!("Standard rate is {r}%, current SG GST is {GST_STANDARD_RATE}% — intended?"),
                    ),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn totals(c: &mut Ctx, doc: &Document) {
    let t = &doc.totals;
    c.require("UBL-TOTAL", "LegalMonetaryTotal/PayableAmount (BT-115)", &t.payable);

    // EN16931 BR-CO-15: tax-inclusive = tax-exclusive + total tax.
    if let (Some(excl), Some(incl), Some(tax)) =
        (amount(&t.tax_exclusive), amount(&t.tax_inclusive), amount(&doc.tax_total_amount))
    {
        if !money_eq(incl, excl + tax) {
            c.err(
                "EN16931-CO-15",
                format!(
                    "TaxInclusiveAmount ({incl:.2}) != TaxExclusiveAmount ({excl:.2}) + TaxTotal ({tax:.2}) = {:.2}",
                    excl + tax
                ),
            );
        }
    }

    // Line extension total should equal the sum of the lines.
    if let Some(declared) = amount(&t.line_extension) {
        let summed: f64 = doc.lines.iter().filter_map(|l| amount(&l.line_extension)).sum();
        if !doc.lines.is_empty() && !money_eq(declared, summed) {
            c.err(
                "EN16931-CO-10",
                format!("LineExtensionAmount ({declared:.2}) != sum of line amounts ({summed:.2})"),
            );
        }
    }
}

fn lines(c: &mut Ctx, doc: &Document) {
    if doc.lines.is_empty() {
        c.err("EN16931-LINE", "The document has no invoice lines");
        return;
    }
    for (i, l) in doc.lines.iter().enumerate() {
        let n = i + 1;
        if l.id.as_deref().unwrap_or("").is_empty() {
            c.err("UBL-LINE-ID", format!("Line {n}: missing line ID"));
        }
        if l.item_name.as_deref().unwrap_or("").is_empty() {
            c.err("EN16931-LINE-NAME", format!("Line {n}: item has no Name (BT-153)"));
        }
        if amount(&l.line_extension).is_none() {
            c.err("EN16931-LINE-AMT", format!("Line {n}: missing or non-numeric LineExtensionAmount (BT-131)"));
        }
        if l.quantity.as_deref().unwrap_or("").is_empty() {
            c.err("UBL-LINE-QTY", format!("Line {n}: missing quantity"));
        }
    }
}
