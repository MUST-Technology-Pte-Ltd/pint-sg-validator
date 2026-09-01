//! UBL -> [`Document`] extraction.
//!
//! roxmltree gives a read-only DOM; we walk it by local name and ignore
//! namespace prefixes (they vary between senders) while still recording the
//! namespace URIs for diagnostics. Nothing here judges the document — parsing
//! only lifts fields out; [`crate::rules`] decides what is wrong.

use crate::model::*;
use std::collections::BTreeMap;

/// Find the first descendant with the given local name.
fn first<'a>(node: roxmltree::Node<'a, 'a>, local: &str) -> Option<roxmltree::Node<'a, 'a>> {
    node.descendants().find(|n| n.is_element() && n.tag_name().name() == local)
}

/// Direct-ish child by local name: the first descendant is fine for the
/// single-valued header fields, but for party/total blocks we scope the search
/// to a subtree, so this helper takes an explicit root.
fn text_of(node: roxmltree::Node, local: &str) -> Option<String> {
    first(node, local)
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_party(root: roxmltree::Node) -> Party {
    let mut p = Party::default();
    if let Some(ep) = first(root, "EndpointID") {
        p.endpoint.scheme = ep.attribute("schemeID").map(|s| s.to_string());
        p.endpoint.value = ep.text().map(|t| t.trim().to_string()).filter(|s| !s.is_empty());
    }
    // PartyName/Name, or fall back to PartyLegalEntity/RegistrationName.
    p.name = first(root, "PartyName")
        .and_then(|n| text_of(n, "Name"))
        .or_else(|| text_of(root, "RegistrationName"));
    if let Some(pts) = first(root, "PartyTaxScheme") {
        p.tax_company_id = text_of(pts, "CompanyID");
        p.tax_scheme_id = first(pts, "TaxScheme").and_then(|ts| text_of(ts, "ID"));
    }
    p
}

pub fn parse(xml: &str) -> Result<Document, String> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| format!("not well-formed XML: {e}"))?;
    let root = doc.root_element();

    let kind = match root.tag_name().name() {
        "Invoice" => DocKind::Invoice,
        "CreditNote" => DocKind::CreditNote,
        other => {
            return Err(format!(
                "root element is <{other}>, expected <Invoice> or <CreditNote>"
            ))
        }
    };

    let mut namespaces = BTreeMap::new();
    for ns in root.namespaces() {
        namespaces.insert(
            ns.name().unwrap_or("(default)").to_string(),
            ns.uri().to_string(),
        );
    }

    // Header fields are unique at the top level.
    let customization_id = text_of(root, "CustomizationID");
    let profile_id = text_of(root, "ProfileID");
    let id = text_of(root, "ID");
    let issue_date = text_of(root, "IssueDate");
    let type_code = text_of(root, "InvoiceTypeCode").or_else(|| text_of(root, "CreditNoteTypeCode"));
    let currency = text_of(root, "DocumentCurrencyCode");

    let supplier = first(root, "AccountingSupplierParty")
        .map(parse_party)
        .unwrap_or_default();
    let customer = first(root, "AccountingCustomerParty")
        .map(parse_party)
        .unwrap_or_default();

    // Tax: the TaxTotal/TaxAmount plus each subtotal's category + percent.
    let (mut tax_total_amount, mut tax_subtotals) = (None, Vec::new());
    if let Some(tt) = first(root, "TaxTotal") {
        tax_total_amount = text_of(tt, "TaxAmount");
        for st in tt.descendants().filter(|n| n.tag_name().name() == "TaxSubtotal") {
            let category = first(st, "TaxCategory").and_then(|c| text_of(c, "ID"));
            let percent = first(st, "TaxCategory").and_then(|c| text_of(c, "Percent"));
            tax_subtotals.push(TaxSubtotal { category, percent });
        }
    }

    let mut totals = MonetaryTotal::default();
    if let Some(lmt) = first(root, "LegalMonetaryTotal") {
        totals.line_extension = text_of(lmt, "LineExtensionAmount");
        totals.tax_exclusive = text_of(lmt, "TaxExclusiveAmount");
        totals.tax_inclusive = text_of(lmt, "TaxInclusiveAmount");
        totals.payable = text_of(lmt, "PayableAmount");
    }

    let line_tag = match kind {
        DocKind::Invoice => "InvoiceLine",
        DocKind::CreditNote => "CreditNoteLine",
    };
    let qty_tag = match kind {
        DocKind::Invoice => "InvoicedQuantity",
        DocKind::CreditNote => "CreditedQuantity",
    };
    let mut lines = Vec::new();
    for ln in root.descendants().filter(|n| n.tag_name().name() == line_tag) {
        lines.push(Line {
            id: text_of(ln, "ID"),
            quantity: text_of(ln, qty_tag),
            line_extension: text_of(ln, "LineExtensionAmount"),
            item_name: first(ln, "Item").and_then(|i| text_of(i, "Name")),
            price_amount: first(ln, "Price").and_then(|p| text_of(p, "PriceAmount")),
        });
    }

    Ok(Document {
        kind,
        customization_id,
        profile_id,
        id,
        issue_date,
        type_code,
        currency,
        supplier,
        customer,
        tax_total_amount,
        tax_subtotals,
        totals,
        lines,
        namespaces,
    })
}
