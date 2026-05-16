#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![cfg(feature = "generate-svg")]

mod common;

use eulumdat_core::{EulumdatError, PlanePair, PolarDiagramOptions, ReportOptions, Symmetry};

#[test]
fn polar_svg_contains_curves_and_labels() {
    let model = common::synthetic_model(
        Symmetry::C0C180,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    let svg = model
        .to_polar_svg(&PolarDiagramOptions::default())
        .expect("polar SVG should render");

    assert!(svg.contains("<svg"));
    assert!(svg.contains("polar-curves"));
    assert!(svg.contains("C0-C180"));
    assert!(svg.contains("C90-C270"));
    assert!(!svg.contains("NaN"));
    assert!(!svg.contains("inf"));
}

#[test]
fn rotational_symmetry_renders_default_plane_pairs() {
    let model = common::synthetic_model(
        Symmetry::Rotational,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    let svg = model
        .to_polar_svg(&PolarDiagramOptions::default())
        .expect("rotational polar SVG should render");

    assert!(svg.contains("C0-C180"));
    assert!(svg.contains("C90-C270"));
}

#[test]
fn unresolved_custom_planes_are_reported() {
    let model = common::synthetic_model(
        Symmetry::None,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    let options = PolarDiagramOptions {
        planes: vec![PlanePair::Custom {
            a: 12.0,
            b: 192.0,
            label: "custom".to_string(),
        }],
        ..PolarDiagramOptions::default()
    };
    let error = model
        .to_polar_svg(&options)
        .expect_err("unavailable custom planes should fail when no curve can render");

    assert!(matches!(error, EulumdatError::Generation(_)));
}

#[test]
fn report_svg_contains_key_datasheet_fields() {
    let model = common::synthetic_model(
        Symmetry::C0C180,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    let svg = model
        .to_report_svg(&ReportOptions::default())
        .expect("report SVG should render");

    assert!(svg.contains("Synthetic luminaire"));
    assert!(svg.contains("Photometry"));
    assert!(svg.contains("Polar diagram"));
    assert!(svg.contains("Synthetic LED"));
}
