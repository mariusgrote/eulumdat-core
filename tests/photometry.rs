#![allow(clippy::pedantic)]
#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
mod common;

use std::f64::consts::PI;

use eulumdat_core::{Eulumdat, Symmetry};

#[test]
fn total_output_and_downward_flux_are_finite() {
    let ldt = common::synthetic_model(
        Symmetry::C0C180AndC90C270,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    assert!(ldt.total_output() > 0.0);
    let downward = ldt.calculated_downward_flux_fraction();
    assert!(downward.is_finite());
    assert!((0.0..=100.0).contains(&downward));
}

#[test]
fn beam_and_field_angle_calculation() {
    let lambertian_field_angle = 2.0 * 0.1_f64.acos() * 180.0 / PI;
    let cosine =
        common::model_with_distribution(common::lambertian_distribution(Symmetry::Rotational));
    common::assert_approx_eq(cosine.beam_angle_c0_c180().unwrap(), 120.0, 0.5);
    common::assert_approx_eq(cosine.beam_angle_c90_c270().unwrap(), 120.0, 0.5);
    common::assert_approx_eq(
        cosine.field_angle_c0_c180().unwrap(),
        lambertian_field_angle,
        0.5,
    );
    common::assert_approx_eq(
        cosine.field_angle_c90_c270().unwrap(),
        lambertian_field_angle,
        0.5,
    );

    let triangular = common::model_with_distribution(common::triangular_distribution(
        Symmetry::Rotational,
        30.0,
    ));
    common::assert_approx_eq(triangular.beam_angle_c0_c180().unwrap(), 30.0, 0.5);
    common::assert_approx_eq(triangular.field_angle_c0_c180().unwrap(), 54.0, 0.5);

    let asymmetric = common::model_with_distribution(common::asymmetric_distribution(
        Symmetry::None,
        30.0,
        60.0,
    ));
    common::assert_approx_eq(asymmetric.beam_angle_c0_c180().unwrap(), 30.0, 0.5);
    common::assert_approx_eq(asymmetric.beam_angle_c90_c270().unwrap(), 60.0, 0.5);
    common::assert_approx_eq(asymmetric.field_angle_c0_c180().unwrap(), 54.0, 0.5);
    common::assert_approx_eq(asymmetric.field_angle_c90_c270().unwrap(), 108.0, 0.5);

    let all_zero = common::model_with_distribution(common::zero_distribution(Symmetry::Rotational));
    assert_eq!(all_zero.beam_angle_c0_c180(), None);
    assert_eq!(all_zero.field_angle_c90_c270(), None);

    let symmetry1 = common::model_with_distribution(common::asymmetric_distribution(
        Symmetry::Rotational,
        30.0,
        60.0,
    ));
    common::assert_approx_eq(
        symmetry1.beam_angle_c0_c180().unwrap(),
        symmetry1.beam_angle_c90_c270().unwrap(),
        1e-9,
    );

    let symmetry2 = common::model_with_distribution(common::asymmetric_distribution(
        Symmetry::C0C180,
        30.0,
        60.0,
    ));
    common::assert_approx_eq(symmetry2.beam_angle_c0_c180().unwrap(), 30.0, 0.5);
    common::assert_approx_eq(symmetry2.beam_angle_c90_c270().unwrap(), 60.0, 0.5);

    let symmetry3 = common::model_with_distribution(common::asymmetric_distribution(
        Symmetry::C90C270,
        30.0,
        60.0,
    ));
    common::assert_approx_eq(symmetry3.beam_angle_c0_c180().unwrap(), 30.0, 0.5);
    common::assert_approx_eq(symmetry3.beam_angle_c90_c270().unwrap(), 60.0, 0.5);

    let symmetry4 = common::model_with_distribution(common::asymmetric_distribution(
        Symmetry::C0C180AndC90C270,
        30.0,
        60.0,
    ));
    common::assert_approx_eq(symmetry4.beam_angle_c0_c180().unwrap(), 30.0, 0.5);
    common::assert_approx_eq(symmetry4.beam_angle_c90_c270().unwrap(), 60.0, 0.5);

    let (reloaded, _) = Eulumdat::parse(&asymmetric.to_text()).expect("roundtrip should parse");
    common::assert_approx_eq(
        reloaded.beam_angle_c0_c180().unwrap(),
        asymmetric.beam_angle_c0_c180().unwrap(),
        1e-9,
    );
    common::assert_approx_eq(
        reloaded.field_angle_c90_c270().unwrap(),
        asymmetric.field_angle_c90_c270().unwrap(),
        1e-9,
    );
}
