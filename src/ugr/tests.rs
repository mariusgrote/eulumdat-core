use std::f64::consts::PI;
use std::fs;

use super::ugr_reference::{SAMPLE_COUNT, fixture_dir, read_sample_text};
use crate::{Distribution, Eulumdat, LampSet, Symmetry};

const MAX_RELATIVE_DEVIATION: f64 = 0.005;

pub(super) fn load_sample(number: usize) -> Eulumdat {
    Eulumdat::parse(&read_sample_text(number))
        .unwrap_or_else(|error| panic!("sample {number:02} should parse: {error}"))
        .0
}

struct LuminanceSample {
    c_deg: f64,
    gamma_deg: f64,
    luminance: f64,
    projected_area: f64,
}

fn load_luminance_reference(number: usize) -> Vec<LuminanceSample> {
    let path = fixture_dir().join(format!("reference/luminance_{number:02}_python.csv"));
    let text = fs::read_to_string(&path).expect("luminance reference should be readable");
    text.lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let values: Vec<f64> = line
                .split(',')
                .map(|value| value.trim().parse().expect("reference value is a number"))
                .collect();
            assert_eq!(values.len(), 4, "unexpected reference row {line:?}");
            LuminanceSample {
                c_deg: values[0],
                gamma_deg: values[1],
                luminance: values[2],
                projected_area: values[3],
            }
        })
        .collect()
}

fn assert_relative(actual: f64, expected: f64, context: &str) {
    let deviation = if expected == 0.0 {
        actual.abs()
    } else {
        ((actual - expected) / expected).abs()
    };
    assert!(
        deviation <= MAX_RELATIVE_DEVIATION,
        "{context}: actual={actual} expected={expected} deviation={deviation}"
    );
}

/// Model with I(C, gamma) = C + gamma / 10 on a coarse, asymmetric grid.
fn linear_model() -> Eulumdat {
    let c_planes = vec![0.0, 90.0, 180.0, 270.0];
    let gamma_angles = vec![0.0, 90.0, 180.0];
    let intensities = c_planes
        .iter()
        .map(|c| gamma_angles.iter().map(|gamma| c + gamma / 10.0).collect())
        .collect();
    let mut model = Eulumdat {
        luminous_area_length: 1000.0,
        luminous_area_width: 500.0,
        conversion_factor: 1.0,
        lamps: vec![LampSet {
            lamp_count: 1,
            lamp_type: "Synthetic LED".to_string(),
            total_luminous_flux: 1000.0,
            color_temperature: "4000K".to_string(),
            color_rendering_index: "80".to_string(),
            wattage_including_ballast: 10.0,
        }],
        ..Eulumdat::default()
    };
    model
        .replace_distribution(Distribution {
            symmetry: Symmetry::None,
            c_plane_step: 90.0,
            gamma_step: 90.0,
            c_planes,
            gamma_angles,
            intensities,
        })
        .expect("synthetic distribution shape should be valid");
    model
}

#[test]
fn luminance_and_projected_area_match_python_reference() {
    for number in 1..=SAMPLE_COUNT {
        let model = load_sample(number);
        let reference = load_luminance_reference(number);
        assert!(
            !reference.is_empty(),
            "sample {number} has no reference rows"
        );
        for sample in reference {
            let context = format!(
                "sample {number:02} at C={} gamma={}",
                sample.c_deg, sample.gamma_deg
            );
            let luminance = model
                .luminance_at(sample.c_deg, sample.gamma_deg)
                .unwrap_or_else(|| panic!("{context}: luminance should be defined"));
            assert_relative(luminance, sample.luminance, &format!("{context} luminance"));
            assert_relative(
                model.projected_area(sample.c_deg, sample.gamma_deg),
                sample.projected_area,
                &format!("{context} projected area"),
            );
        }
    }
}

#[test]
fn intensity_at_returns_stored_values_on_grid() {
    let model = load_sample(1);
    for (c_idx, c_plane) in model.c_planes.iter().enumerate() {
        let profile = model
            .intensity_profile_for_c_plane(*c_plane)
            .expect("expanded C-plane should resolve to a stored row");
        for (gamma_idx, gamma) in model.gamma_angles.iter().enumerate() {
            assert_eq!(
                model.intensity_at(*c_plane, *gamma),
                Some(profile[gamma_idx]),
                "C index {c_idx}, gamma index {gamma_idx}"
            );
        }
    }
}

#[test]
fn intensity_at_interpolates_bilinearly() {
    let model = linear_model();
    let at = |c, gamma| model.intensity_at(c, gamma).unwrap();
    assert!((at(45.0, 45.0) - 49.5).abs() < 1e-9);
    assert!((at(135.0, 135.0) - 148.5).abs() < 1e-9);
    // Between C270 and the wrapped C0 plane.
    assert!((at(315.0, 0.0) - 135.0).abs() < 1e-9);
    assert!((at(337.5, 90.0) - (67.5 + 9.0)).abs() < 1e-9);
}

#[test]
fn c_wraps_around_modulo_360() {
    for number in [1, 4, 5, 11] {
        let model = load_sample(number);
        for gamma in [0.0, 42.5, 65.0, 85.0] {
            assert_eq!(
                model.intensity_at(360.0, gamma),
                model.intensity_at(0.0, gamma)
            );
            assert_eq!(
                model.luminance_at(360.0, gamma),
                model.luminance_at(0.0, gamma)
            );
            let wrapped = model.luminance_at(-12.5, gamma).unwrap();
            let positive = model.luminance_at(347.5, gamma).unwrap();
            assert!((wrapped - positive).abs() <= 1e-9 * positive.abs());
            let far = model.intensity_at(720.0 + 30.0, gamma).unwrap();
            assert!((far - model.intensity_at(30.0, gamma).unwrap()).abs() < 1e-9);
        }
    }

    let model = linear_model();
    assert_eq!(model.intensity_at(360.0, 90.0), Some(9.0));
    assert_eq!(model.intensity_at(-90.0, 90.0), Some(279.0));
}

#[test]
fn rotational_symmetry_is_independent_of_c() {
    let model = load_sample(5);
    assert_eq!(model.symmetry, Symmetry::Rotational);
    assert_eq!(model.luminous_area_width, 0.0);
    for gamma in [0.0, 12.5, 45.0, 67.0, 85.0, 180.0] {
        let intensity = model.intensity_at(0.0, gamma).unwrap();
        let luminance = model.luminance_at(0.0, gamma).unwrap();
        for step in 0..48 {
            let c = f64::from(step) * 7.5 + 1.25;
            assert_eq!(model.intensity_at(c, gamma), Some(intensity), "C={c}");
            let value = model.luminance_at(c, gamma).unwrap();
            assert!(
                (value - luminance).abs() <= 1e-12 * luminance.abs(),
                "C={c} gamma={gamma}: {value} != {luminance}"
            );
        }
    }
}

#[test]
fn circular_luminous_area_uses_diameter_and_c0_height() {
    let mut model = linear_model();
    model.luminous_area_length = 400.0;
    model.luminous_area_width = 0.0;
    model.luminous_area_height_c0 = 50.0;
    model.luminous_area_height_c90 = 999.0;
    model.luminous_area_height_c180 = 999.0;
    model.luminous_area_height_c270 = 999.0;

    let bottom = PI * 0.2 * 0.2;
    let side = 0.4 * 0.05;
    for c in [0.0, 45.0, 90.0, 200.0, 359.0] {
        assert!((model.projected_area(c, 0.0) - bottom).abs() < 1e-12);
        assert!((model.projected_area(c, 90.0) - side).abs() < 1e-12);
        let gamma = 60.0_f64.to_radians();
        let expected = bottom * gamma.cos() + side * gamma.sin();
        assert!((model.projected_area(c, 60.0) - expected).abs() < 1e-12);
    }
}

#[test]
fn rectangular_side_area_uses_quadrant_heights() {
    let mut model = linear_model();
    model.luminous_area_height_c0 = 10.0;
    model.luminous_area_height_c90 = 20.0;
    model.luminous_area_height_c180 = 30.0;
    model.luminous_area_height_c270 = 40.0;

    let side = |c: f64| model.projected_area(c, 90.0);
    let diagonal = 45.0_f64.to_radians().cos();
    assert!((side(0.0) - 0.01).abs() < 1e-12);
    assert!((side(90.0) - 0.5 * 0.02).abs() < 1e-12);
    assert!((side(180.0) - 0.03).abs() < 1e-12);
    assert!((side(270.0) - 0.5 * 0.04).abs() < 1e-12);
    assert!((side(45.0) - diagonal * (0.01 + 0.5 * 0.02)).abs() < 1e-12);
    assert!((side(135.0) - diagonal * (0.03 + 0.5 * 0.02)).abs() < 1e-12);
    assert!((side(225.0) - diagonal * (0.03 + 0.5 * 0.04)).abs() < 1e-12);
    assert!((side(315.0) - diagonal * (0.01 + 0.5 * 0.04)).abs() < 1e-12);
    assert!((model.projected_area(0.0, 0.0) - 0.5).abs() < 1e-12);
}

#[test]
fn luminance_is_interpolated_after_dividing_by_projected_area() {
    let mut model = linear_model();
    model.luminous_area_height_c0 = 100.0;
    model.luminous_area_height_c90 = 100.0;

    let grid =
        |c: f64, gamma: f64| model.intensity_at(c, gamma).unwrap() / model.projected_area(c, gamma);
    let expected = (grid(0.0, 0.0) + grid(0.0, 90.0) + grid(90.0, 0.0) + grid(90.0, 90.0)) / 4.0;
    let actual = model.luminance_at(45.0, 45.0).unwrap();
    assert!((actual - expected).abs() <= 1e-12 * expected);

    // Interpolating intensity first would give a different result here.
    let intensity_first =
        model.intensity_at(45.0, 45.0).unwrap() / model.projected_area(45.0, 45.0);
    assert!((actual - intensity_first).abs() > 1e-3 * expected);
}

#[test]
fn luminance_uses_first_lamp_set_flux() {
    let mut model = linear_model();
    let single = model.luminance_at(45.0, 45.0).unwrap();
    let mut second = model.lamps[0].clone();
    second.total_luminous_flux = 5000.0;
    model.lamps.push(second);
    assert_eq!(model.luminance_at(45.0, 45.0), Some(single));

    model.lamps[0].total_luminous_flux = 2000.0;
    let doubled = model.luminance_at(45.0, 45.0).unwrap();
    assert!((doubled - 2.0 * single).abs() <= 1e-12 * doubled);

    model.lamps.clear();
    assert_eq!(model.luminance_at(45.0, 45.0), None);
}

#[test]
fn luminance_uses_conversion_factor_once() {
    let mut model = linear_model();
    let baseline = model.luminance_at(45.0, 45.0).unwrap();

    for factor in [0.5, 2.0] {
        model.conversion_factor = factor;
        let converted = model.luminance_at(45.0, 45.0).unwrap();
        assert!((converted - factor * baseline).abs() <= 1e-12 * baseline);
    }
}

#[test]
fn non_positive_projected_area_on_grid_counts_as_zero_luminance() {
    // Flat area without side heights: A_proj is negative above the horizon.
    let model = linear_model();
    assert!(model.projected_area(0.0, 180.0) < 0.0);
    assert_eq!(model.luminance_at(0.0, 180.0), Some(0.0));
    assert_eq!(model.luminance_at(270.0, 180.0), Some(0.0));
}

#[test]
fn zero_luminous_area_has_no_luminance() {
    let mut model = linear_model();
    model.luminous_area_width = 0.0;
    model.luminous_area_length = 0.0;
    model.luminous_area_height_c0 = 50.0;
    assert_eq!(model.projected_area(0.0, 0.0), 0.0);
    assert_eq!(model.luminance_at(0.0, 0.0), None);
    assert_eq!(model.luminance_at(90.0, 65.0), None);

    let mut model = linear_model();
    model.luminous_area_length = 0.0;
    assert_eq!(model.luminance_at(0.0, 65.0), None);
    assert!(model.intensity_at(0.0, 65.0).is_some());
}

#[test]
fn out_of_range_or_missing_data_yields_none() {
    let model = linear_model();
    assert_eq!(model.intensity_at(0.0, -1.0), None);
    assert_eq!(model.intensity_at(0.0, 180.5), None);
    assert_eq!(model.intensity_at(f64::NAN, 10.0), None);
    assert_eq!(model.intensity_at(0.0, f64::NAN), None);
    assert_eq!(model.luminance_at(0.0, 181.0), None);
    assert_eq!(model.intensity_at(0.0, 180.0), Some(18.0));

    assert_eq!(Eulumdat::default().intensity_at(0.0, 0.0), None);
}
