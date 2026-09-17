// UGR table tests. They live inside the crate because `UgrTable` stays
// crate-private until the public API is added in a later phase.

use std::fmt;
use std::fs;
use std::time::Instant;

use super::background::FluxFractions;
use super::guth::position_index;
use super::tests::{SAMPLE_COUNT, fixture_dir, load_sample};
use super::{UGR_REFLECTANCES, UGR_ROOMS, UgrTable};
use crate::{Distribution, Eulumdat, Symmetry};

const PYTHON_TOLERANCE: f64 = 0.05;
const RELUX_TOLERANCE: f64 = 0.5;
const CIE_190_TOLERANCE: f64 = 0.3;
const SYMMETRY_TOLERANCE: f64 = 0.05;

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

/// Parsed reference cell: an exact value, or Relux's "<10.0" upper bound.
#[derive(Debug, Clone, Copy)]
enum Reference {
    Value(f64),
    Below(f64),
}

fn read_reference(name: &str) -> Vec<Vec<Reference>> {
    let path = fixture_dir().join("reference").join(name);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
    let table: Vec<Vec<Reference>> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split(',')
                .map(|cell| {
                    let cell = cell.trim();
                    let parse = |value: &str| -> f64 {
                        value
                            .parse()
                            .unwrap_or_else(|_| panic!("{name}: invalid cell {cell:?}"))
                    };
                    match cell.strip_prefix('<') {
                        Some(bound) => Reference::Below(parse(bound)),
                        None => Reference::Value(parse(cell)),
                    }
                })
                .collect()
        })
        .collect();
    assert_eq!(table.len(), 19, "{name}: row count");
    assert!(table.iter().all(|row| row.len() == 10), "{name}: columns");
    table
}

fn computed(table: &UgrTable, number: usize, row: usize, column: usize) -> f64 {
    table.values[row][column]
        .unwrap_or_else(|| panic!("sample {number:02} cell [{row}][{column}] is not computed"))
}

fn round_to_tenth(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

/// Largest deviation of the table from a reference, with the position.
#[derive(Default, Clone, Copy)]
struct Deviation {
    max: f64,
    number: usize,
    row: usize,
    column: usize,
}

impl fmt::Display for Deviation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:.3} (sample {:02}, row {}, column {})",
            self.max, self.number, self.row, self.column
        )
    }
}

impl Deviation {
    fn record(&mut self, deviation: f64, number: usize, row: usize, column: usize) {
        if deviation > self.max {
            *self = Self {
                max: deviation,
                number,
                row,
                column,
            };
        }
    }
}

#[test]
fn python_parity() {
    let mut overall = Deviation::default();
    for number in 1..=SAMPLE_COUNT {
        if GAMMA_90_CORRECTED_SAMPLES.contains(&number) {
            continue;
        }
        let model = load_sample(number);
        assert!(
            model.gamma_angles.first() == Some(&0.0) && model.gamma_angles.last() >= Some(&175.0),
            "sample {number:02} does not cover all zone midpoints; \
             list it in GAMMA_90_CORRECTED_SAMPLES"
        );
        let table = model.ugr_table();
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
    for number in 1..=SAMPLE_COUNT {
        let table = load_sample(number).ugr_table();
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
    for number in 1..=SAMPLE_COUNT {
        let table = load_sample(number).ugr_table();
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
fn sample_11_matches_cie_190() {
    // CIE 190:2010 computes its example table with luminaire spacing 1.0·H,
    // not the catalogue spacing 0.25·H (eulumdat-ugr validates the same way).
    // The catalogue table deviates from it by up to about 0.75.
    let model = load_sample(11);
    let reference = read_reference("ugr_table_11_cie190.csv");
    for (spacing, tolerance) in [(1.0, Some(CIE_190_TOLERANCE)), (0.25, None)] {
        let table = model.ugr_table_with_spacing(spacing);
        let mut overall = Deviation::default();
        for (row, cells) in reference.iter().enumerate() {
            for (column, cell) in cells.iter().enumerate() {
                let Reference::Value(expected) = *cell else {
                    panic!("CIE 190 reference has no bounds");
                };
                let actual = computed(&table, 11, row, column);
                let deviation = (actual - expected).abs();
                if let Some(tolerance) = tolerance {
                    assert!(
                        deviation <= tolerance,
                        "[{row}][{column}]: {actual} vs CIE 190 {expected}"
                    );
                }
                overall.record(deviation, 11, row, column);
            }
        }
        println!("max deviation from CIE 190 at S = {spacing}·H: {overall}");
    }
}

#[test]
fn doubling_flux_adds_8_log10_2() {
    let expected = 8.0 * 2.0_f64.log10();
    for number in [1, 5, 11] {
        let mut model = load_sample(number);
        let single = model.ugr_table();
        model.lamps[0].total_luminous_flux *= 2.0;
        let doubled = model.ugr_table();
        assert_eq!(doubled.lamp_flux, 2.0 * single.lamp_flux);
        for row in 0..19 {
            for column in 0..10 {
                let difference = computed(&doubled, number, row, column)
                    - computed(&single, number, row, column);
                assert!(
                    (difference - expected).abs() < 1e-9,
                    "sample {number:02} [{row}][{column}]: +{difference}"
                );
            }
        }
    }
}

#[test]
fn rotational_symmetry_gives_equal_orientations() {
    for number in [5, 8, 9] {
        let model = load_sample(number);
        assert_eq!(model.symmetry, Symmetry::Rotational, "sample {number:02}");
        assert_eq!(model.luminous_area_width, 0.0, "sample {number:02}");
        let table = model.ugr_table();
        for row in 0..19 {
            for column in 0..5 {
                let crosswise = computed(&table, number, row, column);
                let endwise = computed(&table, number, row, column + 5);
                assert!(
                    (crosswise - endwise).abs() <= SYMMETRY_TOLERANCE,
                    "sample {number:02} row {row}: {crosswise} vs {endwise}"
                );
            }
        }
    }
}

#[test]
fn upper_zones_without_data_have_no_flux() {
    // Sample 11 emits 2.58 cd/klm at 90° and nothing above. Cut at 90°, the
    // file must keep its flux fractions and UGR table. eulumdat-ugr would
    // continue the 90° intensity into the 95°..175° zones instead.
    let full = load_sample(11);
    let lower = truncated_at_90(&full);
    assert_eq!(FluxFractions::new(&lower), FluxFractions::new(&full));
    assert_eq!(lower.ugr_table(), full.ugr_table());

    // Sample 4 has upward light; cut at 90° it matches the file with every
    // intensity above 90° set to zero (both use a 5° grid).
    let full = load_sample(4);
    let mut dark_above = full.clone();
    for row in &mut dark_above.intensities {
        for (value, gamma) in row.iter_mut().zip(&full.gamma_angles) {
            if *gamma > 90.0 {
                *value = 0.0;
            }
        }
    }
    let lower = truncated_at_90(&full);
    assert_eq!(FluxFractions::new(&lower), FluxFractions::new(&dark_above));
    assert_ne!(FluxFractions::new(&lower), FluxFractions::new(&full));
    assert!(
        lower
            .ugr_table()
            .values
            .iter()
            .flatten()
            .all(Option::is_some)
    );
}

/// `ldt` with the distribution cut after the 90° gamma angle.
fn truncated_at_90(ldt: &Eulumdat) -> Eulumdat {
    let cut = ldt
        .gamma_angles
        .iter()
        .position(|gamma| *gamma == 90.0)
        .expect("sample has a 90° gamma angle");
    let mut model = ldt.clone();
    model
        .replace_distribution(Distribution {
            symmetry: ldt.symmetry,
            c_plane_step: ldt.c_plane_step,
            gamma_step: ldt.gamma_step,
            c_planes: ldt.c_planes.clone(),
            gamma_angles: ldt.gamma_angles[..=cut].to_vec(),
            intensities: ldt
                .intensities
                .iter()
                .map(|row| row[..=cut].to_vec())
                .collect(),
        })
        .expect("truncated distribution should be valid");
    model
}

#[test]
fn missing_lamps_or_distribution_yield_empty_table() {
    let mut model = load_sample(1);
    model.lamps.clear();
    let table = model.ugr_table();
    assert_eq!(table.lamp_flux, 0.0);
    assert!(table.values.iter().flatten().all(Option::is_none));

    let table = Eulumdat::default().ugr_table();
    assert!(table.values.iter().flatten().all(Option::is_none));
}

#[test]
fn constants_follow_cie_190_order() {
    assert_eq!(UGR_ROOMS[0], (2, 2));
    assert_eq!(UGR_ROOMS[10], (4, 8));
    assert_eq!(UGR_ROOMS[18], (12, 8));
    assert_eq!(UGR_REFLECTANCES[0], (0.7, 0.5, 0.2));
    assert_eq!(UGR_REFLECTANCES[4], (0.3, 0.3, 0.2));
    for (room, &(x, y)) in UGR_ROOMS.iter().enumerate() {
        let k = f64::from(x) * f64::from(y) / f64::from(x + y);
        assert!(
            (super::tables::ROOM_K[room] - k).abs() < 0.006,
            "room {room}: k"
        );
        assert!(
            super::tables::F_GL
                .iter()
                .any(|(table_k, _)| *table_k == super::tables::ROOM_K[room]),
            "room {room}: F_GL"
        );
    }
}

#[test]
#[ignore = "timing; run with `cargo test --release -- --ignored --nocapture ugr_table_timing`"]
fn ugr_table_timing() {
    const ROUNDS: u32 = 20;
    let models: Vec<Eulumdat> = (1..=SAMPLE_COUNT).map(load_sample).collect();
    let start = Instant::now();
    for _ in 0..ROUNDS {
        for model in &models {
            std::hint::black_box(model.ugr_table());
        }
    }
    let per_table = start.elapsed() / (ROUNDS * SAMPLE_COUNT as u32);
    println!("UGR table: {per_table:?} per table");
}

// Guth position index samples from eulumdat-ugr tests/test_ugr.py
// (`TestGuthTable`, `TestGuthPVec`).

fn assert_index(h_r: f64, t_r: f64, expected: f64) {
    let actual = position_index(h_r, t_r);
    assert!(
        (actual - expected).abs() <= 1e-6 * expected,
        "p(H/R={h_r}, T/R={t_r}) = {actual}, expected {expected}"
    );
}

#[test]
fn guth_grid_points() {
    assert_index(0.0, 0.0, 1.00);
    assert_index(0.50, 0.0, 2.86);
    assert_index(0.50, 0.50, 2.91);
    assert_index(1.00, 1.00, 7.00);
    assert_index(1.90, 3.00, 16.00);
    assert_index(0.00, 1.00, 2.11);
    assert_index(0.30, 1.30, 3.70);
    assert_index(1.50, 2.00, 12.85);
    // Published outlier, kept as-is.
    assert_index(0.60, 2.70, 7.50);
}

#[test]
fn guth_bilinear_interpolation() {
    assert_index(0.55, 0.55, (2.91 + 3.10 + 3.40 + 3.60) / 4.0);
    assert_index(0.05, 0.0, 1.13);
    assert_index(0.0, 0.05, 1.025);

    let corners = [(0.30, 1.10), (0.30, 1.20), (0.40, 1.10), (0.40, 1.20)]
        .map(|(h_r, t_r)| position_index(h_r, t_r));
    let value = position_index(0.35, 1.15);
    let low = corners.iter().copied().fold(f64::INFINITY, f64::min);
    let high = corners.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!((low..=high).contains(&value));
}

#[test]
fn guth_uses_absolute_ratios() {
    assert_eq!(position_index(0.50, -1.00), position_index(0.50, 1.00));
    assert_eq!(position_index(-0.80, 1.50), position_index(0.80, 1.50));
}

#[test]
fn guth_missing_or_out_of_range_is_nan() {
    // Missing cell.
    assert!(position_index(1.90, 0.0).is_nan());
    // Outside the table.
    assert!(position_index(2.00, 1.00).is_nan());
    assert!(position_index(1.00, 3.10).is_nan());
    assert!(position_index(5.00, 5.00).is_nan());
    assert!(position_index(5.0, 0.0).is_nan());
    // One of the four surrounding cells is missing.
    assert!(position_index(1.75, 0.05).is_nan());
    assert!(position_index(f64::NAN, 1.0).is_nan());
}

#[test]
fn guth_nan_neighbour_poisons_exact_grid_points() {
    // scipy picks the cell [x_i, x_i+1) and multiplies the NaN upper corner by
    // a zero weight, which still yields NaN.
    assert!(position_index(1.70, 0.0).is_nan());
    assert!(position_index(1.80, 0.40).is_nan());
    // At the upper table edge the cell is [x_n-2, x_n-1].
    assert_index(1.90, 0.90, 16.0);
    assert_index(1.85, 0.90, 15.5);
}
