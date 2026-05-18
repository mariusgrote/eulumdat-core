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
