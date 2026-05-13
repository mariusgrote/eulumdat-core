#![allow(clippy::pedantic)]
#![allow(dead_code)]

use eulumdat_core::{Distribution, Eulumdat, LampSet, Symmetry, TypeIndicator};

pub fn synthetic_ldt(symmetry: Symmetry, type_indicator: TypeIndicator) -> String {
    synthetic_model(symmetry, type_indicator).to_text()
}

pub fn synthetic_model(symmetry: Symmetry, type_indicator: TypeIndicator) -> Eulumdat {
    let distribution = basic_distribution(symmetry);
    let mut ldt = Eulumdat {
        identification: "Independent synthetic fixture".to_string(),
        type_indicator,
        measurement_report_number: "SYN-REPORT-001".to_string(),
        luminaire_name: "Synthetic luminaire".to_string(),
        luminaire_number: "SYN-001".to_string(),
        file_name: "synthetic.ldt".to_string(),
        date_user: "2026-05-13 test".to_string(),
        luminaire_length: 100.0,
        luminaire_width: 100.0,
        luminaire_height: 50.0,
        luminous_area_length: 80.0,
        luminous_area_width: 80.0,
        luminous_area_height_c0: 0.0,
        luminous_area_height_c90: 0.0,
        luminous_area_height_c180: 0.0,
        luminous_area_height_c270: 0.0,
        downward_flux_fraction: 50.0,
        light_output_ratio: 100.0,
        conversion_factor: 1.0,
        tilt: 0.0,
        lamps: vec![LampSet {
            lamp_count: 1,
            lamp_type: "Synthetic LED".to_string(),
            total_luminous_flux: 1000.0,
            color_temperature: "4000K".to_string(),
            color_rendering_index: "80".to_string(),
            wattage_including_ballast: 10.0,
        }],
        direct_ratios: [0.0; 10],
        ..Eulumdat::default()
    };
    ldt.replace_distribution(distribution)
        .expect("synthetic distribution shape should be valid");
    ldt
}

pub fn basic_distribution(symmetry: Symmetry) -> Distribution {
    let c_planes = c_planes_for(symmetry);
    let gamma_angles = vec![0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0];
    let rows = symmetry
        .stored_c_plane_count(c_planes.len())
        .expect("synthetic stored C-plane count");
    let intensities = (0..rows)
        .map(|row| {
            gamma_angles
                .iter()
                .enumerate()
                .map(|(index, gamma)| {
                    (1000.0 - gamma * 3.0 + row as f64 * 25.0 + index as f64).max(10.0)
                })
                .collect()
        })
        .collect();
    Distribution {
        symmetry,
        c_plane_step: if c_planes.len() > 1 { 90.0 } else { 0.0 },
        gamma_step: 30.0,
        c_planes,
        gamma_angles,
        intensities,
    }
}

pub fn lambertian_distribution(symmetry: Symmetry) -> Distribution {
    distribution_from_columns(
        symmetry,
        vec![lambertian_column(); stored_count_for_test(symmetry)],
    )
}

pub fn triangular_distribution(symmetry: Symmetry, beam_width: f64) -> Distribution {
    distribution_from_columns(
        symmetry,
        vec![beam_angle_column(beam_width); stored_count_for_test(symmetry)],
    )
}

pub fn asymmetric_distribution(symmetry: Symmetry, narrow: f64, wide: f64) -> Distribution {
    let narrow = beam_angle_column(narrow);
    let wide = beam_angle_column(wide);
    let columns = match symmetry {
        Symmetry::None => vec![narrow.clone(), wide.clone(), narrow, wide],
        Symmetry::Rotational => vec![wide],
        Symmetry::C0C180 => vec![narrow.clone(), wide, narrow],
        Symmetry::C90C270 => vec![wide.clone(), narrow, wide],
        Symmetry::C0C180AndC90C270 => vec![narrow, wide],
    };
    distribution_from_columns(symmetry, columns)
}

pub fn zero_distribution(symmetry: Symmetry) -> Distribution {
    distribution_from_columns(
        symmetry,
        vec![vec![0.0; beam_angle_gamma_values().len()]; stored_count_for_test(symmetry)],
    )
}

pub fn model_with_distribution(distribution: Distribution) -> Eulumdat {
    let mut model = synthetic_model(
        distribution.symmetry,
        TypeIndicator::PointSourceWithSymmetry,
    );
    model
        .replace_distribution(distribution)
        .expect("synthetic distribution shape should be valid");
    model
}

pub fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual} expected={expected} tolerance={tolerance}"
    );
}

pub fn table_text(c_planes: &[f64]) -> String {
    let gamma = [0.0, 45.0, 90.0, 135.0, 180.0];
    let mut text = String::from("\t");
    for (index, c) in c_planes.iter().enumerate() {
        if index > 0 {
            text.push('\t');
        }
        text.push_str(&format!("{c}"));
    }
    text.push('\n');
    for (g_index, g) in gamma.iter().enumerate() {
        text.push_str(&format!("{g}"));
        for c_index in 0..c_planes.len() {
            text.push_str(&format!("\t{}", 10 + g_index + c_index));
        }
        text.push('\n');
    }
    text
}

fn distribution_from_columns(symmetry: Symmetry, intensities: Vec<Vec<f64>>) -> Distribution {
    Distribution {
        symmetry,
        c_plane_step: 90.0,
        gamma_step: 5.0,
        c_planes: c_planes_for(symmetry),
        gamma_angles: beam_angle_gamma_values(),
        intensities,
    }
}

fn c_planes_for(symmetry: Symmetry) -> Vec<f64> {
    match symmetry {
        Symmetry::Rotational => vec![0.0],
        Symmetry::None | Symmetry::C0C180 | Symmetry::C90C270 | Symmetry::C0C180AndC90C270 => {
            vec![0.0, 90.0, 180.0, 270.0]
        }
    }
}

fn stored_count_for_test(symmetry: Symmetry) -> usize {
    symmetry
        .stored_c_plane_count(c_planes_for(symmetry).len())
        .expect("synthetic stored C-plane count")
}

fn beam_angle_gamma_values() -> Vec<f64> {
    (0..=180).step_by(5).map(f64::from).collect()
}

fn beam_angle_column(beam_angle: f64) -> Vec<f64> {
    beam_angle_gamma_values()
        .into_iter()
        .map(|gamma| 1000.0 * (1.0 - gamma / beam_angle).max(0.0))
        .collect()
}

fn lambertian_column() -> Vec<f64> {
    beam_angle_gamma_values()
        .into_iter()
        .map(|gamma| 1000.0 * gamma.to_radians().cos().max(0.0))
        .collect()
}
