#![allow(clippy::pedantic)]
#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]

// UGR table through the public API: golden tests against the reference tables
// and the blockers of the tabular method.

#[path = "common/ugr_reference.rs"]
mod ugr_reference;

use eulumdat_core::{
    Distribution, Eulumdat, FluxBasis, LampSet, Symmetry, UGR_REFLECTANCES, UGR_ROOMS, UgrBlocker,
    UgrRoom, UgrTable, UgrView, ValidationSettings,
};
use ugr_reference::{
    Deviation, Reference, SAMPLE_COUNT, read_reference, read_sample_text, round_to_tenth,
};

const PYTHON_TOLERANCE: f64 = 0.05;
const RELUX_TOLERANCE: f64 = 0.5;
const SYMMETRY_TOLERANCE: f64 = 0.05;

/// Samples the tabular method correctly refuses.
///
/// Sample 06 emits 80 % of its flux upwards (the file header states 21 %
/// downward). Relux and DIALux still publish a table, but LiTG Publ. 20 limits
/// the tabular method to an upward share of 65 %. The crate-internal test
/// `blocked_sample_06_matches_references` keeps its golden comparison.
const BLOCKED_SAMPLES: &[usize] = &[6];

/// Samples 01 to 11 whose tables are held against references.
fn table_samples() -> impl Iterator<Item = usize> {
    (1..=SAMPLE_COUNT).filter(|number| !BLOCKED_SAMPLES.contains(number))
}

/// Samples whose Python reference values are affected by the deliberate γ > 90°
/// deviation in `FluxFractions::new` and are therefore not held to
/// [`PYTHON_TOLERANCE`].
///
/// Empty: every sample stores γ from 0° to 180°, so all 18 zone midpoints lie
/// inside the measured range and both implementations use the same zonal
/// fluxes. `python_parity` checks this for each sample it holds to the
/// tolerance, so a new sample that only covers γ ≤ 90° fails until it is
/// listed here with a reason.
const GAMMA_90_CORRECTED_SAMPLES: &[usize] = &[];

fn load_sample(number: usize) -> Eulumdat {
    Eulumdat::parse(&read_sample_text(number))
        .unwrap_or_else(|error| panic!("sample {number:02} should parse: {error}"))
        .0
}

fn table(model: &Eulumdat, context: &str) -> UgrTable {
    model
        .ugr_table()
        .unwrap_or_else(|blockers| panic!("{context} is blocked: {blockers:?}"))
}

/// Maps a reference column to its view and reflectance index.
fn cell(column: usize) -> (UgrView, usize) {
    if column < UGR_REFLECTANCES.len() {
        (UgrView::Crosswise, column)
    } else {
        (UgrView::Endwise, column - UGR_REFLECTANCES.len())
    }
}

fn computed(table: &UgrTable, number: usize, row: usize, column: usize) -> f64 {
    let (view, reflectance) = cell(column);
    table
        .value(row, view, reflectance, FluxBasis::LampFlux)
        .unwrap_or_else(|| panic!("sample {number:02} cell [{row}][{column}] is not computed"))
}

#[test]
fn reference_samples_are_not_blocked() {
    for number in table_samples() {
        let table = table(&load_sample(number), &format!("sample {number:02}"));
        for row in table.rows(FluxBasis::LampFlux) {
            assert!(
                row.crosswise
                    .iter()
                    .chain(&row.endwise)
                    .all(Option::is_some),
                "sample {number:02} room {:?} has empty cells",
                row.room
            );
        }
    }
}

#[test]
fn sample_06_is_blocked_for_its_upward_share() {
    let blockers = load_sample(6).ugr_table().unwrap_err();
    let [UgrBlocker::IndirectShareTooHigh { upward_fraction }] = blockers[..] else {
        panic!("unexpected blockers {blockers:?}");
    };
    assert!((0.79..0.81).contains(&upward_fraction), "{upward_fraction}");
}

#[test]
fn python_parity() {
    let mut overall = Deviation::default();
    for number in table_samples() {
        if GAMMA_90_CORRECTED_SAMPLES.contains(&number) {
            continue;
        }
        let model = load_sample(number);
        assert!(
            model.gamma_angles.first() == Some(&0.0) && model.gamma_angles.last() >= Some(&175.0),
            "sample {number:02} does not cover all zone midpoints; \
             list it in GAMMA_90_CORRECTED_SAMPLES"
        );
        let table = table(&model, &format!("sample {number:02}"));
        let reference = read_reference(&format!("ugr_table_{number:02}_python.csv"));
        for (row, cells) in reference.iter().enumerate() {
            for (column, cell) in cells.iter().enumerate() {
                let Reference::Value(expected) = *cell else {
                    panic!("python reference has no bounds");
                };
                let actual = computed(&table, number, row, column);
                let deviation = (actual - expected).abs();
                assert!(
                    deviation <= PYTHON_TOLERANCE,
                    "sample {number:02} [{row}][{column}]: {actual} vs python {expected}"
                );
                overall.record(deviation, number, row, column);
            }
        }
    }
    println!("max deviation from Python: {overall}");
}

#[test]
fn golden_relux() {
    let mut overall = Deviation::default();
    for number in table_samples() {
        let table = table(&load_sample(number), &format!("sample {number:02}"));
        let reference = read_reference(&format!("ugr_table_{number:02}_Relux.csv"));
        for (row, cells) in reference.iter().enumerate() {
            for (column, cell) in cells.iter().enumerate() {
                let actual = round_to_tenth(computed(&table, number, row, column));
                let deviation = match *cell {
                    Reference::Value(expected) => (actual - expected).abs(),
                    Reference::Below(bound) => (actual - bound).max(0.0),
                };
                assert!(
                    deviation <= RELUX_TOLERANCE,
                    "sample {number:02} [{row}][{column}]: {actual} vs Relux {cell:?}"
                );
                overall.record(deviation, number, row, column);
            }
        }
    }
    println!("max deviation from Relux: {overall}");
}

#[test]
fn report_dialux_deviation() {
    // DIALux deviates by up to about 1.0 UGR, so this only reports.
    let mut overall = Deviation::default();
    for number in table_samples() {
        let table = table(&load_sample(number), &format!("sample {number:02}"));
        let reference = read_reference(&format!("ugr_table_{number:02}_Dialux.csv"));
        let mut sample = Deviation::default();
        for (row, cells) in reference.iter().enumerate() {
            for (column, cell) in cells.iter().enumerate() {
                let Reference::Value(expected) = *cell else {
                    panic!("DIALux reference has no bounds");
                };
                let actual = round_to_tenth(computed(&table, number, row, column));
                let deviation = (actual - expected).abs();
                sample.record(deviation, number, row, column);
                overall.record(deviation, number, row, column);
            }
        }
        println!(
            "sample {number:02}: max deviation from DIALux {:.1}",
            sample.max
        );
    }
    println!("max deviation from DIALux: {overall}");
}

#[test]
fn doubling_flux_adds_8_log10_2() {
    let expected = 8.0 * 2.0_f64.log10();
    for number in [1, 5, 11] {
        let mut model = load_sample(number);
        let single = table(&model, "single flux");
        model.lamps[0].total_luminous_flux *= 2.0;
        let doubled = table(&model, "doubled flux");
        assert_eq!(doubled.lamp_flux(), 2.0 * single.lamp_flux());
        for row in 0..UGR_ROOMS.len() {
            for column in 0..10 {
                let difference = computed(&doubled, number, row, column)
                    - computed(&single, number, row, column);
                assert!(
                    (difference - expected).abs() < 1e-9,
                    "sample {number:02} [{row}][{column}]: +{difference}"
                );
            }
        }
        // Normalized to 1000 lm, both tables are the same.
        assert!(
            single
                .rows(FluxBasis::Normalized1000Lm)
                .zip(doubled.rows(FluxBasis::Normalized1000Lm))
                .all(|(a, b)| a
                    .crosswise
                    .iter()
                    .chain(&a.endwise)
                    .zip(b.crosswise.iter().chain(&b.endwise))
                    .all(|(a, b)| (a.unwrap() - b.unwrap()).abs() < 1e-9))
        );
    }
}

#[test]
fn rotational_symmetry_gives_equal_views() {
    for number in [5, 8, 9] {
        let model = load_sample(number);
        assert_eq!(model.symmetry, Symmetry::Rotational, "sample {number:02}");
        assert_eq!(model.luminous_area_width, 0.0, "sample {number:02}");
        let table = table(&model, &format!("sample {number:02}"));
        for row in table.rows(FluxBasis::LampFlux) {
            for (crosswise, endwise) in row.crosswise.iter().zip(&row.endwise) {
                let (crosswise, endwise) = (crosswise.unwrap(), endwise.unwrap());
                assert!(
                    (crosswise - endwise).abs() <= SYMMETRY_TOLERANCE,
                    "sample {number:02} room {:?}: {crosswise} vs {endwise}",
                    row.room
                );
            }
        }
    }
}

#[test]
fn normalized_basis_subtracts_flux_correction() {
    let table = table(&load_sample(4), "sample 04");
    let flux = table.lamp_flux();
    assert!(flux != 1000.0);
    assert!((table.flux_correction() - 8.0 * (flux / 1000.0).log10()).abs() < 1e-12);
    for room in 0..UGR_ROOMS.len() {
        for view in [UgrView::Crosswise, UgrView::Endwise] {
            for reflectance in 0..UGR_REFLECTANCES.len() {
                let lamp = table.value(room, view, reflectance, FluxBasis::LampFlux);
                let normalized = table.value(room, view, reflectance, FluxBasis::Normalized1000Lm);
                assert_eq!(normalized, lamp.map(|v| v - table.flux_correction()));
            }
        }
    }
}

#[test]
fn rows_match_values_in_room_order() {
    let table = table(&load_sample(1), "sample 01");
    for basis in [FluxBasis::LampFlux, FluxBasis::Normalized1000Lm] {
        let rows: Vec<_> = table.rows(basis).collect();
        assert_eq!(rows.len(), UGR_ROOMS.len());
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.room, UGR_ROOMS[index]);
            for reflectance in 0..UGR_REFLECTANCES.len() {
                let value = |view| table.value(index, view, reflectance, basis);
                assert_eq!(row.crosswise[reflectance], value(UgrView::Crosswise));
                assert_eq!(row.endwise[reflectance], value(UgrView::Endwise));
            }
        }
    }
}

#[test]
fn data_sheet_value_is_room_4h_8h_at_70_50_20() {
    let table = table(&load_sample(1), "sample 01");
    let room = UGR_ROOMS
        .iter()
        .position(|room| *room == UgrRoom { x_h: 4, y_h: 8 })
        .unwrap();
    assert_eq!(
        (
            UGR_REFLECTANCES[0].ceiling,
            UGR_REFLECTANCES[0].walls,
            UGR_REFLECTANCES[0].floor
        ),
        (0.7, 0.5, 0.2)
    );
    for basis in [FluxBasis::LampFlux, FluxBasis::Normalized1000Lm] {
        let (crosswise, endwise) = table.data_sheet_value(basis);
        assert!(crosswise.is_some() && endwise.is_some());
        assert_eq!(crosswise, table.value(room, UgrView::Crosswise, 0, basis));
        assert_eq!(endwise, table.value(room, UgrView::Endwise, 0, basis));
        let row = table.rows(basis).nth(room).unwrap();
        assert_eq!((crosswise, endwise), (row.crosswise[0], row.endwise[0]));
    }
}

#[test]
fn value_out_of_range_is_none() {
    let table = table(&load_sample(1), "sample 01");
    let basis = FluxBasis::LampFlux;
    assert!(table.value(18, UgrView::Endwise, 4, basis).is_some());
    assert_eq!(table.value(19, UgrView::Crosswise, 0, basis), None);
    assert_eq!(table.value(0, UgrView::Crosswise, 5, basis), None);
}

// Blockers. Each constructed model starts from `valid_model` and breaks one
// assumption of the tabular method.

/// Grid and intensity function for a synthetic distribution without symmetry.
struct Grid {
    c_step: f64,
    gamma_start: f64,
    gamma_end: f64,
    gamma_step: f64,
}

const FINE_GRID: Grid = Grid {
    c_step: 15.0,
    gamma_start: 0.0,
    gamma_end: 180.0,
    gamma_step: 5.0,
};

fn steps(start: f64, end: f64, step: f64) -> Vec<f64> {
    let count = ((end - start) / step).round() as usize;
    (0..=count).map(|i| start + i as f64 * step).collect()
}

/// Mostly downward cosine distribution with a little upward light.
fn downlight(_c: f64, gamma: f64) -> f64 {
    1000.0 * gamma.to_radians().cos().max(0.0) + 20.0
}

fn model(grid: &Grid, intensity: impl Fn(f64, f64) -> f64) -> Eulumdat {
    let c_planes = steps(0.0, 360.0 - grid.c_step, grid.c_step);
    let gamma_angles = steps(grid.gamma_start, grid.gamma_end, grid.gamma_step);
    let intensities = c_planes
        .iter()
        .map(|&c| gamma_angles.iter().map(|&g| intensity(c, g)).collect())
        .collect();
    let mut model = Eulumdat {
        luminaire_length: 620.0,
        luminaire_width: 620.0,
        luminaire_height: 60.0,
        luminous_area_length: 600.0,
        luminous_area_width: 600.0,
        luminous_area_height_c0: 20.0,
        luminous_area_height_c90: 20.0,
        luminous_area_height_c180: 20.0,
        luminous_area_height_c270: 20.0,
        downward_flux_fraction: 95.0,
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
            symmetry: Symmetry::None,
            c_plane_step: grid.c_step,
            gamma_step: grid.gamma_step,
            c_planes,
            gamma_angles,
            intensities,
        })
        .expect("synthetic distribution shape should be valid");
    model
}

fn valid_model() -> Eulumdat {
    model(&FINE_GRID, downlight)
}

fn blockers(model: &Eulumdat) -> Vec<UgrBlocker> {
    model.ugr_table().expect_err("model should be blocked")
}

#[test]
fn valid_synthetic_model_is_not_blocked() {
    let model = valid_model();
    assert!(model.validate(ValidationSettings::unrestricted()).is_ok());
    let table = table(&model, "valid model");
    assert!(table.rows(FluxBasis::LampFlux).all(|row| {
        row.crosswise
            .iter()
            .chain(&row.endwise)
            .all(Option::is_some)
    }));
}

#[test]
fn zero_luminous_area_blocks() {
    let mut model = valid_model();
    model.luminous_area_length = 0.0;
    assert_eq!(blockers(&model), [UgrBlocker::NoLuminousArea]);

    // A circular area needs a positive diameter.
    let mut model = valid_model();
    model.luminous_area_width = 0.0;
    assert!(model.ugr_table().is_ok());
    model.luminous_area_length = -10.0;
    assert_eq!(blockers(&model), [UgrBlocker::NoLuminousArea]);
}

#[test]
fn missing_lamp_flux_blocks() {
    let mut model = valid_model();
    model.lamps[0].total_luminous_flux = 0.0;
    assert_eq!(blockers(&model), [UgrBlocker::NoLampFlux]);

    model.lamps.clear();
    assert_eq!(blockers(&model), [UgrBlocker::NoLampFlux]);
}

#[test]
fn zero_light_output_ratio_blocks() {
    let mut model = valid_model();
    model.light_output_ratio = 0.0;
    assert_eq!(blockers(&model), [UgrBlocker::NoLightOutputRatio]);
}

#[test]
fn invalid_distribution_blocks() {
    let mut model = valid_model();
    model.intensities[3].pop();
    assert_eq!(blockers(&model), [UgrBlocker::InvalidDistribution]);

    let mut model = valid_model();
    model.gamma_angles.swap(1, 2);
    assert_eq!(blockers(&model), [UgrBlocker::InvalidDistribution]);

    let mut model = valid_model();
    model
        .intensities
        .iter_mut()
        .flatten()
        .for_each(|v| *v = 0.0);
    assert_eq!(blockers(&model), [UgrBlocker::InvalidDistribution]);
}

#[test]
fn gamma_range_below_90_blocks() {
    let upper = model(
        &Grid {
            gamma_end: 80.0,
            ..FINE_GRID
        },
        downlight,
    );
    assert_eq!(
        blockers(&upper),
        [UgrBlocker::IncompleteDistribution {
            min_gamma: Some(0.0),
            max_gamma: Some(80.0),
        }]
    );

    let lower = model(
        &Grid {
            gamma_start: 10.0,
            ..FINE_GRID
        },
        downlight,
    );
    assert_eq!(
        blockers(&lower),
        [UgrBlocker::IncompleteDistribution {
            min_gamma: Some(10.0),
            max_gamma: Some(180.0),
        }]
    );

    // The lower hemisphere alone is enough.
    let hemisphere = model(
        &Grid {
            gamma_end: 90.0,
            ..FINE_GRID
        },
        downlight,
    );
    assert!(hemisphere.ugr_table().is_ok());
}

#[test]
fn coarse_angle_grid_blocks() {
    let coarse_gamma = model(
        &Grid {
            gamma_step: 10.0,
            ..FINE_GRID
        },
        downlight,
    );
    assert_eq!(
        blockers(&coarse_gamma),
        [UgrBlocker::AngleGridTooCoarse {
            c_step: Some(15.0),
            gamma_step: 10.0,
        }]
    );

    let coarse_c = model(
        &Grid {
            c_step: 30.0,
            ..FINE_GRID
        },
        downlight,
    );
    assert_eq!(
        blockers(&coarse_c),
        [UgrBlocker::AngleGridTooCoarse {
            c_step: Some(30.0),
            gamma_step: 5.0,
        }]
    );

    // The actual planes count, not the header step.
    let mut uneven = valid_model();
    assert!(uneven.ugr_table().is_ok());
    uneven.c_plane_step = 90.0;
    uneven.gamma_step = 30.0;
    assert!(uneven.ugr_table().is_ok());
    uneven.c_planes.remove(2);
    uneven.intensities.remove(2);
    assert_eq!(
        blockers(&uneven),
        [UgrBlocker::AngleGridTooCoarse {
            c_step: Some(30.0),
            gamma_step: 5.0,
        }]
    );

    // Rotational symmetry has no C-planes to check.
    let mut rotational = model(
        &Grid {
            gamma_step: 10.0,
            ..FINE_GRID
        },
        downlight,
    );
    let row = rotational.intensities[0].clone();
    rotational
        .replace_distribution(Distribution {
            symmetry: Symmetry::Rotational,
            c_plane_step: 0.0,
            gamma_step: 10.0,
            c_planes: vec![0.0],
            gamma_angles: rotational.gamma_angles.clone(),
            intensities: vec![row],
        })
        .unwrap();
    assert_eq!(
        blockers(&rotational),
        [UgrBlocker::AngleGridTooCoarse {
            c_step: None,
            gamma_step: 10.0,
        }]
    );
}

#[test]
fn high_upward_share_blocks() {
    let model = model(&FINE_GRID, |_, gamma| {
        let cos = gamma.to_radians().cos();
        if cos >= 0.0 {
            300.0 * cos
        } else {
            -1000.0 * cos
        }
    });
    let blockers = blockers(&model);
    let [UgrBlocker::IndirectShareTooHigh { upward_fraction }] = blockers[..] else {
        panic!("unexpected blockers {blockers:?}");
    };
    assert!(
        (upward_fraction - 1000.0 / 1300.0).abs() < 0.01,
        "{upward_fraction}"
    );
    // The header value is ignored.
    assert_eq!(model.downward_flux_fraction, 95.0);
}

#[test]
fn asymmetric_distribution_blocks() {
    let model = model(&FINE_GRID, |c, gamma| {
        downlight(c, gamma) * (1.0 + 0.3 * c.to_radians().cos())
    });
    let blockers = blockers(&model);
    let [UgrBlocker::Asymmetric { max_deviation }] = blockers[..] else {
        panic!("unexpected blockers {blockers:?}");
    };
    // At C = 0°, γ = 0°: I = 1020·1.3 against I(C180) = 1020·0.7.
    assert!((max_deviation - 0.6 / 1.3).abs() < 1e-9, "{max_deviation}");
}

#[test]
fn measurement_noise_is_not_asymmetry() {
    // Symmetric data stored without symmetry, with ±2 % deterministic noise.
    let model = model(&FINE_GRID, |c, gamma| {
        downlight(c, gamma) * (1.0 + 0.02 * (c * 7.0 + gamma * 13.0).to_radians().sin())
    });
    assert!(model.ugr_table().is_ok());
}

#[test]
fn all_blockers_are_reported_together() {
    let mut model = model(
        &Grid {
            gamma_end: 80.0,
            gamma_step: 10.0,
            ..FINE_GRID
        },
        |c, gamma| downlight(c, gamma) * (1.0 + 0.3 * c.to_radians().cos()),
    );
    model.luminous_area_length = 0.0;
    model.lamps.clear();
    model.light_output_ratio = 0.0;
    let blockers = blockers(&model);
    assert!(matches!(
        blockers[..],
        [
            UgrBlocker::NoLuminousArea,
            UgrBlocker::NoLampFlux,
            UgrBlocker::NoLightOutputRatio,
            UgrBlocker::IncompleteDistribution {
                min_gamma: Some(0.0),
                max_gamma: Some(80.0),
            },
            UgrBlocker::AngleGridTooCoarse {
                c_step: Some(15.0),
                gamma_step: 10.0,
            },
            UgrBlocker::Asymmetric { .. },
        ]
    ));
}

#[test]
fn blockers_display_short_messages() {
    let blockers = [
        UgrBlocker::NoLuminousArea,
        UgrBlocker::NoLampFlux,
        UgrBlocker::NoLightOutputRatio,
        UgrBlocker::InvalidDistribution,
        UgrBlocker::IncompleteDistribution {
            min_gamma: Some(0.0),
            max_gamma: Some(80.0),
        },
        UgrBlocker::IncompleteDistribution {
            min_gamma: None,
            max_gamma: None,
        },
        UgrBlocker::AngleGridTooCoarse {
            c_step: Some(30.0),
            gamma_step: 5.0,
        },
        UgrBlocker::AngleGridTooCoarse {
            c_step: None,
            gamma_step: 10.0,
        },
        UgrBlocker::IndirectShareTooHigh {
            upward_fraction: 0.802,
        },
        UgrBlocker::Asymmetric {
            max_deviation: 0.123,
        },
    ];
    let messages: Vec<String> = blockers.iter().map(ToString::to_string).collect();
    for message in &messages {
        assert!(message.contains(" -> ") && message.len() < 100, "{message}");
    }
    assert_eq!(messages[4], "Gamma angles = 0°..80° -> must cover 0°..90°");
    assert_eq!(
        messages[6],
        "Angle steps = gamma 5°, C 30° -> too coarse (max gamma 5°, C 15°)"
    );
    assert_eq!(messages[8], "Upward flux fraction = 80.2 % -> above 65 %");
    assert_eq!(
        messages[9],
        "Asymmetry = 12.3 % of peak intensity -> above 5 %"
    );
}
