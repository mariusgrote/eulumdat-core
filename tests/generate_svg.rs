#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![cfg(feature = "generate-svg")]

mod common;

use eulumdat_core::{
    EulumdatError, PlanePair, PolarDiagramOptions, PolarDiagramPresentation, ReportOptions,
    Symmetry,
};

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
    assert!(svg.contains("data-polar-presentation=\"classic\""));
    assert!(!svg.contains("polar-fills"));
    assert!(!svg.contains("NaN"));
    assert!(!svg.contains("inf"));
}

#[test]
fn focused_presentation_crops_downlights_and_fills_each_curve() {
    let model =
        common::model_with_distribution(common::lambertian_distribution(Symmetry::Rotational));
    let options = PolarDiagramOptions {
        width: 520,
        height: 520,
        presentation: PolarDiagramPresentation::Focused,
        ..PolarDiagramOptions::default()
    };
    let svg = model
        .to_polar_svg(&options)
        .expect("focused polar SVG should render");

    assert!(svg.contains("data-polar-framing=\"downlight\""));
    assert!(svg.contains("id=\"polar-fills\""));
    assert_eq!(svg.matches("fill=\"#fff28a\"").count(), 2);
    assert!(svg.contains("stroke=\"#ff6b6b\""));
    assert!(svg.contains("stroke=\"#7375ff\""));
    assert!(svg.contains("30°"));
    assert!(!svg.contains("15°"));
    assert!(svg.contains("cd/klm"));
}

#[test]
fn focused_presentation_shows_fifteen_degree_labels_when_large() {
    let model =
        common::model_with_distribution(common::lambertian_distribution(Symmetry::Rotational));
    let options = PolarDiagramOptions {
        width: 900,
        height: 900,
        presentation: PolarDiagramPresentation::Focused,
        ..PolarDiagramOptions::default()
    };
    let svg = model
        .to_polar_svg(&options)
        .expect("large focused polar SVG should render");

    assert!(svg.contains("15°"));
    assert!(svg.contains("75°"));
}

#[test]
fn focused_presentation_retains_full_view_for_uplight() {
    let model = common::synthetic_model(
        Symmetry::C0C180,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    let options = PolarDiagramOptions {
        presentation: PolarDiagramPresentation::Focused,
        ..PolarDiagramOptions::default()
    };
    let svg = model
        .to_polar_svg(&options)
        .expect("focused uplight SVG should render");

    assert!(svg.contains("data-polar-framing=\"full\""));
    assert!(svg.contains("180°"));
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
    assert!(svg.contains("data-polar-presentation=\"classic\""));
    assert!(!svg.contains("polar-fills"));
}
