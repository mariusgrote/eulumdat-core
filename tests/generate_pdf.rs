#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![cfg(feature = "generate-pdf")]

mod common;

use eulumdat_core::{ReportOptions, Symmetry};

#[test]
fn report_pdf_has_pdf_header() {
    let model = common::synthetic_model(
        Symmetry::C0C180,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    let pdf = model
        .to_report_pdf(&ReportOptions::default())
        .expect("PDF should render");

    assert!(pdf.starts_with(b"%PDF"));
}

#[test]
fn report_pdf_embeds_dejavu_sans_font() {
    let model = common::synthetic_model(
        Symmetry::C0C180,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    let pdf = model
        .to_report_pdf(&ReportOptions::default())
        .expect("PDF should render");

    assert!(
        contains_subsequence(&pdf, b"DejaVuSans"),
        "expected the PDF to embed the DejaVu Sans font that ships with the crate, \
         which proves text was actually rendered (otherwise no font is referenced)"
    );
}

fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}
