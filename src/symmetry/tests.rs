use crate::{Distribution, Eulumdat, FluxBasis, LampSet, Symmetry, TypeIndicator, UgrView};

const TOLERANCE: f64 = 1e-9;

/// Intensity in cd/klm, symmetric to the C90–C270 plane.
///
/// Depends on C only through sin C (unchanged by C → 180° − C) and cos² C, so
/// C270 (980), C0 (1300), and C90 (1020) have distinct amplitudes. C90 and
/// C270 differ by 3 % of the peak, within the UGR asymmetry limit.
fn intensity(c_deg: f64, gamma_deg: f64) -> f64 {
    let c = c_deg.to_radians();
    let amplitude = 1000.0 + 20.0 * c.sin() + 300.0 * c.cos().powi(2);
    amplitude * gamma_deg.to_radians().cos().max(0.0)
}

fn gamma_angles() -> Vec<f64> {
    (0..=180).step_by(5).map(f64::from).collect()
}

fn equidistant_c_planes() -> Vec<f64> {
    (0..360).step_by(15).map(f64::from).collect()
}

/// C-planes symmetric to C90–C270 with uneven gaps.
fn uneven_c_planes() -> Vec<f64> {
    vec![
        0.0, 10.0, 30.0, 90.0, 150.0, 170.0, 180.0, 190.0, 210.0, 270.0, 330.0, 350.0,
    ]
}

fn profile(c_deg: f64) -> Vec<f64> {
    gamma_angles()
        .into_iter()
        .map(|gamma| intensity(c_deg, gamma))
        .collect()
}

/// ISYM 3 model whose rows are written out explicitly in EULUMDAT order.
fn stored_model(c_planes: Vec<f64>, stored_c_angles: &[f64]) -> Eulumdat {
    model(
        Symmetry::C90C270,
        c_planes,
        stored_c_angles.iter().map(|c| profile(*c)).collect(),
    )
}

/// ISYM 0 model with one row per C-plane.
fn expanded_model(c_planes: Vec<f64>) -> Eulumdat {
    let intensities = c_planes.iter().map(|c| profile(*c)).collect();
    model(Symmetry::None, c_planes, intensities)
}

fn model(symmetry: Symmetry, c_planes: Vec<f64>, intensities: Vec<Vec<f64>>) -> Eulumdat {
    let mut model = Eulumdat {
        identification: "ISYM 3 orientation".to_string(),
        type_indicator: TypeIndicator::LinearLuminaire,
        luminaire_name: "Synthetic luminaire".to_string(),
        luminaire_length: 600.0,
        luminaire_width: 300.0,
        luminaire_height: 50.0,
        luminous_area_length: 600.0,
        luminous_area_width: 300.0,
        downward_flux_fraction: 100.0,
        light_output_ratio: 80.0,
        conversion_factor: 1.0,
        lamps: vec![LampSet {
            lamp_count: 1,
            lamp_type: "Synthetic LED".to_string(),
            total_luminous_flux: 3000.0,
            color_temperature: "4000K".to_string(),
            color_rendering_index: "80".to_string(),
            wattage_including_ballast: 25.0,
        }],
        ..Eulumdat::default()
    };
    model
        .replace_distribution(Distribution {
            symmetry,
            c_plane_step: 0.0,
            gamma_step: 5.0,
            c_planes,
            gamma_angles: gamma_angles(),
            intensities,
        })
        .expect("synthetic distribution shape should be valid");
    model
}

/// C270, C285, …, C345, C0, C15, …, C90.
fn equidistant_stored_c_angles() -> Vec<f64> {
    (0..=12).map(|k| f64::from(270 + 15 * k) % 360.0).collect()
}

fn equidistant_stored() -> Eulumdat {
    stored_model(equidistant_c_planes(), &equidistant_stored_c_angles())
}

fn uneven_stored() -> Eulumdat {
    stored_model(
        uneven_c_planes(),
        &[270.0, 330.0, 350.0, 0.0, 10.0, 30.0, 90.0],
    )
}

fn assert_profile(model: &Eulumdat, c_deg: f64, expected: &[f64]) {
    let actual = model
        .intensity_profile_for_c_plane(c_deg)
        .unwrap_or_else(|| panic!("C{c_deg} should resolve to a stored row"));
    assert_eq!(actual.len(), expected.len(), "C{c_deg}");
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= TOLERANCE,
            "C{c_deg} gamma index {index}: {actual} vs {expected}"
        );
    }
}

#[test]
fn representative_angles_follow_each_symmetry() {
    let cases = [
        (Symmetry::None, -15.0, 345.0),
        (Symmetry::None, 375.0, 15.0),
        (Symmetry::Rotational, 123.0, 0.0),
        (Symmetry::C0C180, 195.0, 165.0),
        (Symmetry::C0C180, 90.0, 90.0),
        (Symmetry::C90C270, 0.0, 0.0),
        (Symmetry::C90C270, 90.0, 90.0),
        (Symmetry::C90C270, 270.0, 270.0),
        (Symmetry::C90C270, 165.0, 15.0),
        (Symmetry::C90C270, 195.0, 345.0),
        (Symmetry::C90C270, 180.0, 0.0),
        (Symmetry::C90C270, -90.0, 270.0),
        (Symmetry::C90C270, 450.0, 90.0),
        (Symmetry::C0C180AndC90C270, 195.0, 15.0),
        (Symmetry::C0C180AndC90C270, 300.0, 60.0),
    ];
    for (symmetry, c, expected) in cases {
        let actual = symmetry.representative_c_angle(c);
        assert!(
            (actual - expected).abs() <= TOLERANCE,
            "{symmetry:?} C{c}: {actual} vs {expected}"
        );
    }
}

#[test]
fn c90_c270_rows_start_at_c270() {
    let model = equidistant_stored();
    assert_eq!(
        model.c90_c270_row_offsets(),
        (0..=12).map(|k| f64::from(15 * k)).collect::<Vec<_>>()
    );
    for (c, row) in [
        (270.0, 0),
        (345.0, 5),
        (0.0, 6),
        (360.0, 6),
        (15.0, 7),
        (90.0, 12),
    ] {
        assert_eq!(model.stored_row_for_c_plane(c), Some(row), "C{c}");
    }
    assert_eq!(model.stored_row_for_c_plane(7.5), None);
    assert_eq!(model.stored_row_for_c_plane(f64::NAN), None);
}

/// Fails with the former mapping, which read row 0 as C90 and row 12 as C270.
#[test]
fn exact_c270_c0_c90_profiles() {
    let model = equidistant_stored();
    assert_profile(&model, 270.0, &profile(270.0));
    assert_profile(&model, 0.0, &profile(0.0));
    assert_profile(&model, 90.0, &profile(90.0));
    assert_profile(&model, -90.0, &profile(270.0));
    assert_profile(&model, 360.0, &profile(0.0));
    assert_profile(&model, 180.0, &profile(0.0));

    let gamma = 0.0;
    assert!((model.intensity_at(270.0, gamma).unwrap() - 980.0).abs() <= TOLERANCE);
    assert!((model.intensity_at(0.0, gamma).unwrap() - 1300.0).abs() <= TOLERANCE);
    assert!((model.intensity_at(90.0, gamma).unwrap() - 1020.0).abs() <= TOLERANCE);
}

#[test]
fn mirrored_planes_share_their_profile() {
    for model in [equidistant_stored(), uneven_stored()] {
        for (stored, mirrored) in [(15.0, 165.0), (345.0, 195.0), (30.0, 150.0), (330.0, 210.0)] {
            if model.stored_row_for_c_plane(stored).is_none() {
                continue;
            }
            assert_profile(&model, stored, &profile(stored));
            assert_profile(&model, mirrored, &profile(stored));
        }
    }
    let uneven = uneven_stored();
    assert_profile(&uneven, 190.0, &profile(350.0));
    assert_profile(&uneven, 170.0, &profile(10.0));
}

#[test]
fn interpolation_across_the_360_degree_wrap() {
    let model = equidistant_stored();
    // Gamma angles on the grid keep the interpolation linear in C only.
    for gamma in [0.0, 20.0, 60.0] {
        let at = |c: f64| model.intensity_at(c, gamma).unwrap();
        let mean = |a: f64, b: f64| (intensity(a, gamma) + intensity(b, gamma)) / 2.0;
        assert!(
            (at(352.5) - mean(345.0, 0.0)).abs() <= TOLERANCE,
            "{gamma} {} {}",
            at(352.5),
            mean(345.0, 0.0)
        );
        assert!((at(-7.5) - mean(345.0, 0.0)).abs() <= TOLERANCE);
        assert!((at(7.5) - mean(0.0, 15.0)).abs() <= TOLERANCE);
        let quarter =
            intensity(345.0, gamma) + 0.25 * (intensity(0.0, gamma) - intensity(345.0, gamma));
        assert!((at(348.75) - quarter).abs() <= TOLERANCE);
        // Mirror images of the two points above across C90–C270.
        assert!((at(187.5) - at(352.5)).abs() <= TOLERANCE);
        assert!((at(172.5) - at(7.5)).abs() <= TOLERANCE);
    }

    // Uneven grid: 355° lies halfway between C350 and the wrapped C0 plane, and
    // its mirror image 185° between C180 and C190 resolves to the same rows.
    let uneven = uneven_stored();
    let gamma = 30.0;
    let expected = (intensity(350.0, gamma) + intensity(0.0, gamma)) / 2.0;
    for c in [355.0, -5.0, 185.0] {
        assert!((uneven.intensity_at(c, gamma).unwrap() - expected).abs() <= TOLERANCE);
    }
}

#[test]
fn parse_serialize_parse_keeps_orientation() {
    for (original, stored_c_angles) in [
        (equidistant_stored(), equidistant_stored_c_angles()),
        (
            uneven_stored(),
            vec![270.0, 330.0, 350.0, 0.0, 10.0, 30.0, 90.0],
        ),
    ] {
        let text = original.to_text();

        // The file ends with the rows, which must run from C270 to C90.
        let lines: Vec<&str> = text.lines().collect();
        let gamma_count = original.gamma_count();
        let rows = &lines[lines.len() - stored_c_angles.len() * gamma_count..];
        for (row, c) in rows.chunks(gamma_count).zip(&stored_c_angles) {
            let values: Vec<f64> = row.iter().map(|line| line.parse().unwrap()).collect();
            assert_eq!(values, profile(*c), "row for C{c}");
        }

        let (parsed, _) = Eulumdat::parse(&text).expect("serialized ISYM 3 should parse");
        assert_eq!(parsed, original);
        let (reparsed, _) = Eulumdat::parse(&parsed.to_text()).expect("second parse");
        assert_eq!(parsed.to_text(), text);
        for c in &original.c_planes {
            assert_profile(&reparsed, *c, &profile(*c));
        }
    }
}

#[test]
fn stored_and_expanded_models_agree() {
    for (stored, expanded) in [
        (equidistant_stored(), expanded_model(equidistant_c_planes())),
        (uneven_stored(), expanded_model(uneven_c_planes())),
    ] {
        for c in &expanded.c_planes {
            let reference = expanded.intensity_profile_for_c_plane(*c).unwrap().to_vec();
            assert_profile(&stored, *c, &reference);
        }
        for step in 0..=144 {
            let c = f64::from(step) * 2.5;
            for gamma in [0.0, 12.5, 45.0, 90.0, 135.0] {
                let (a, b) = (
                    stored.intensity_at(c, gamma),
                    expanded.intensity_at(c, gamma),
                );
                assert!(
                    (a.unwrap() - b.unwrap()).abs() <= TOLERANCE,
                    "C{c} gamma {gamma}: {a:?} vs {b:?}"
                );
            }
        }
        for (a, b) in [
            (stored.beam_angle_c0_c180(), expanded.beam_angle_c0_c180()),
            (stored.beam_angle_c90_c270(), expanded.beam_angle_c90_c270()),
            (stored.field_angle_c0_c180(), expanded.field_angle_c0_c180()),
            (
                stored.field_angle_c90_c270(),
                expanded.field_angle_c90_c270(),
            ),
        ] {
            assert!(
                (a.unwrap() - b.unwrap()).abs() <= TOLERANCE,
                "{a:?} vs {b:?}"
            );
        }
    }
}

#[test]
fn stored_and_expanded_models_have_the_same_ugr_table() {
    let stored = equidistant_stored()
        .ugr_table()
        .unwrap_or_else(|blockers| panic!("ISYM 3 model is blocked: {blockers:?}"));
    let expanded = expanded_model(equidistant_c_planes())
        .ugr_table()
        .unwrap_or_else(|blockers| panic!("ISYM 0 model is blocked: {blockers:?}"));

    let mut compared = 0;
    for (a, b) in stored
        .rows(FluxBasis::LampFlux)
        .zip(expanded.rows(FluxBasis::LampFlux))
    {
        for (a, b) in a
            .crosswise
            .iter()
            .chain(&a.endwise)
            .zip(b.crosswise.iter().chain(&b.endwise))
        {
            match (a, b) {
                (Some(a), Some(b)) => {
                    assert!((a - b).abs() <= TOLERANCE, "{a} vs {b}");
                    compared += 1;
                }
                (a, b) => assert_eq!(a, b),
            }
        }
    }
    assert!(compared > 0);
    assert!(
        stored
            .value(0, UgrView::Endwise, 0, FluxBasis::LampFlux)
            .is_some()
    );
}
