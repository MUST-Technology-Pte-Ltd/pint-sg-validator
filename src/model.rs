//! The subset of a UBL billing document this validator reasons about.
//!
//! Only the fields the PINT SG rules touch are lifted out of the XML; the rest
//! of the document is ignored. Everything is optional at parse time so that a
//! missing element becomes a *rule finding* with a clear message, never a parse
//! error the caller has to decode.

use std::collections::BTreeMap;

/// Which UBL document was submitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Invoice,
    CreditNote,
}

impl DocKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DocKind::Invoice => "Invoice",
            DocKind::CreditNote => "CreditNote",
        }
    }
}

/// A party endpoint identifier, e.g. `<cbc:EndpointID schemeID="0195">…`.
#[derive(Debug, Clone, Default)]
pub struct SchemedId {
    pub scheme: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Party {
    pub endpoint: SchemedId,
    pub name: Option<String>,
    /// CompanyID under cac:PartyTaxScheme — the GST registration number.
    pub tax_company_id: Option<String>,
    pub tax_scheme_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TaxSubtotal {
    pub category: Option<String>,
    pub percent: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MonetaryTotal {
    pub line_extension: Option<String>,
    pub tax_exclusive: Option<String>,
    pub tax_inclusive: Option<String>,
    pub payable: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Line {
    pub id: Option<String>,
    pub quantity: Option<String>,
    pub line_extension: Option<String>,
    pub item_name: Option<String>,
    pub price_amount: Option<String>,
}

/// The parsed document. `raw_attrs` keeps a couple of root-level facts that the
/// rules report back to the user verbatim.
#[derive(Debug, Clone)]
pub struct Document {
    pub kind: DocKind,
    pub customization_id: Option<String>,
    pub profile_id: Option<String>,
    pub id: Option<String>,
    pub issue_date: Option<String>,
    pub type_code: Option<String>,
    pub currency: Option<String>,
    pub supplier: Party,
    pub customer: Party,
    pub tax_total_amount: Option<String>,
    pub tax_subtotals: Vec<TaxSubtotal>,
    pub totals: MonetaryTotal,
    pub lines: Vec<Line>,
    /// Namespaces seen on the root, for diagnostics.
    pub namespaces: BTreeMap<String, String>,
}
