use pint_sg_validator::{validate, Severity};

const VALID: &str = include_str!("fixtures/valid_invoice.xml");

#[test]
fn a_conforming_invoice_passes() {
    let r = validate(VALID);
    let errs: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    assert!(r.conforms, "expected conforming, got errors: {errs:?}");
    assert_eq!(r.errors, 0);
}

#[test]
fn wrong_customization_id_is_an_error() {
    let bad = VALID.replace("urn:peppol:pint:billing-1@sg-1", "urn:peppol:pint:billing-1@aus-1");
    let r = validate(&bad);
    assert!(!r.conforms);
    assert!(r.findings.iter().any(|f| f.rule == "PINT-SG-01"));
}

#[test]
fn broken_total_arithmetic_is_caught() {
    // TaxInclusive should be 109.00; corrupt it.
    let bad = VALID.replace(
        "<cbc:TaxInclusiveAmount currencyID=\"SGD\">109.00</cbc:TaxInclusiveAmount>",
        "<cbc:TaxInclusiveAmount currencyID=\"SGD\">119.00</cbc:TaxInclusiveAmount>",
    );
    let r = validate(&bad);
    assert!(r.findings.iter().any(|f| f.rule == "EN16931-CO-15"), "arithmetic rule should fire");
}

#[test]
fn line_sum_mismatch_is_caught() {
    let bad = VALID.replace(
        "<cbc:LineExtensionAmount currencyID=\"SGD\">100.00</cbc:LineExtensionAmount>\n    <cbc:TaxExclusiveAmount",
        "<cbc:LineExtensionAmount currencyID=\"SGD\">200.00</cbc:LineExtensionAmount>\n    <cbc:TaxExclusiveAmount",
    );
    let r = validate(&bad);
    assert!(r.findings.iter().any(|f| f.rule == "EN16931-CO-10"));
}

#[test]
fn missing_endpoint_scheme_flags_the_seller() {
    let bad = VALID.replace("schemeID=\"0195\">SGUEN200406416G", ">SGUEN200406416G");
    let r = validate(&bad);
    assert!(r.findings.iter().any(|f| f.rule == "PINT-SG-SELLER-EPSCHEME"));
}

#[test]
fn malformed_xml_is_one_fatal_finding_not_a_panic() {
    let r = validate("<Invoice><unclosed>");
    assert!(!r.conforms);
    assert_eq!(r.findings.len(), 1);
    assert_eq!(r.findings[0].rule, "XML-PARSE");
}

#[test]
fn wrong_root_element_is_reported() {
    let r = validate("<?xml version=\"1.0\"?><Order xmlns=\"x\"><a/></Order>");
    assert!(!r.conforms);
    assert_eq!(r.findings[0].rule, "XML-PARSE");
}
