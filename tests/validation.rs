#![allow(clippy::pedantic)]
#![allow(unused_crate_dependencies)]
mod common;

use eulumdat_core::{Distribution, Symmetry, TypeIndicator, ValidationSettings};

#[test]
fn validation_detects_distribution_shape_mismatch() {
    let mut ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    ldt.intensities.pop();
    let error = ldt
        .validate(ValidationSettings::unrestricted())
        .unwrap_err()
        .to_string();
    assert!(error.contains("distribution shape"), "{error}");
}

#[test]
fn replace_distribution_rejects_wrong_dimensions() {
    let mut ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    let before = ldt.clone();
    let error = ldt
        .replace_distribution(Distribution {
            symmetry: before.symmetry,
            c_plane_step: before.c_plane_step,
            gamma_step: before.gamma_step,
            c_planes: before.c_planes.clone(),
            gamma_angles: before.gamma_angles.clone(),
            intensities: vec![vec![1.0]],
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("distribution shape"), "{error}");
    assert_eq!(ldt, before);
}

#[test]
fn validation_uses_explicit_settings() {
    let mut ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    ldt.identification = "A".repeat(100);

    let unrestricted = ldt
        .validate(ValidationSettings::unrestricted())
        .expect("unrestricted validation");
    assert!(
        !unrestricted
            .iter()
            .any(|warning| warning.field == "Identification")
    );

    let restricted = ldt
        .validate(ValidationSettings::restricted())
        .expect("restricted validation");
    assert!(
        restricted
            .iter()
            .any(|warning| warning.field == "Identification"
                && warning.message.contains("line is too long"))
    );
}

#[test]
fn validation_hard_errors() {
    let mut ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    ldt.gamma_angles.swap(0, 1);
    assert!(
        ldt.validate(ValidationSettings::unrestricted())
            .unwrap_err()
            .to_string()
            .contains("Gamma-planes not sorted")
    );

    let mut ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    ldt.c_planes.swap(0, 1);
    assert!(
        ldt.validate(ValidationSettings::unrestricted())
            .unwrap_err()
            .to_string()
            .contains("C-planes not sorted")
    );

    let mut ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    for row in &mut ldt.intensities {
        for value in row {
            *value = 0.5;
        }
    }
    assert!(
        ldt.validate(ValidationSettings::unrestricted())
            .unwrap_err()
            .to_string()
            .contains("values are too low")
    );
}
