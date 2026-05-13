#![allow(clippy::pedantic)]
#![allow(unused_crate_dependencies)]
mod common;

use eulumdat_core::{Symmetry, TypeIndicator};

#[test]
fn resample_gamma_valid_steps() {
    for step in [1, 5, 10, 180] {
        let mut ldt =
            common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
        let original_c_planes = ldt.c_plane_count();
        ldt.resample_gamma(step)
            .expect("valid step should resample");
        assert_eq!(ldt.c_plane_count(), original_c_planes);
        assert!(ldt.gamma_count() >= 2);
        assert!(ldt.gamma_angles.windows(2).all(|pair| pair[1] > pair[0]));
    }
}

#[test]
fn resample_gamma_invalid_steps() {
    let mut ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    assert!(ldt.resample_gamma(0).is_err());
    assert!(ldt.resample_gamma(181).is_err());
}

#[test]
fn resample_gamma_keeps_shape() {
    let mut ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    let first_gamma = ldt.gamma_angles[0];
    let last_gamma = ldt.gamma_angles[ldt.gamma_angles.len() - 1];
    ldt.resample_gamma(10).expect("resample should work");
    assert_eq!(ldt.gamma_angles[0], first_gamma);
    assert_eq!(ldt.gamma_angles[ldt.gamma_angles.len() - 1], last_gamma);
    assert_eq!(ldt.intensities.len(), ldt.stored_c_plane_count().unwrap());
    for row in &ldt.intensities {
        assert_eq!(row.len(), ldt.gamma_count());
    }
}

#[test]
fn scale_to_100_handles_zero_output() {
    let mut ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    for row in &mut ldt.intensities {
        for value in row {
            *value = 0.0;
        }
    }
    let before = ldt.clone();
    ldt.scale_to_100_percent();
    assert_eq!(ldt, before);
}

#[test]
fn scale_to_100_with_flux_handles_zero_flux() {
    let mut ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    for row in &mut ldt.intensities {
        for value in row {
            *value = 0.0;
        }
    }
    let before = ldt.clone();
    ldt.scale_to_100_percent_with_flux();
    assert_eq!(ldt, before);
}
