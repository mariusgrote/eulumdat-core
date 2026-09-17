// UGR tests that need crate-internal access. The golden tests against the
// reference tables use the public API and live in tests/ugr_table.rs.

use std::time::Instant;

use super::background::FluxFractions;
use super::geometry::CATALOGUE_SPACING_TO_HEIGHT;
use super::guth::position_index;
use super::tests::load_sample;
use super::ugr_reference::{Deviation, Reference, SAMPLE_COUNT, read_reference, round_to_tenth};
use super::{UGR_REFLECTANCES, UGR_ROOMS, UgrReflectances, UgrRoom, UgrTable, UgrView};
use crate::{Distribution, Eulumdat, FluxBasis};

const CIE_190_TOLERANCE: f64 = 0.3;
const PYTHON_TOLERANCE: f64 = 0.05;
const RELUX_TOLERANCE: f64 = 0.5;

fn computed(table: &UgrTable, number: usize, row: usize, column: usize) -> f64 {
    let (view, reflectance) = if column < 5 {
        (UgrView::Crosswise, column)
    } else {
        (UgrView::Endwise, column - 5)
    };
    table
        .value(row, view, reflectance, FluxBasis::LampFlux)
        .unwrap_or_else(|| panic!("sample {number:02} cell [{row}][{column}] is not computed"))
}

fn all_cells(table: &UgrTable) -> impl Iterator<Item = Option<f64>> + '_ {
    table
        .rows(FluxBasis::LampFlux)
        .flat_map(|row| row.crosswise.into_iter().chain(row.endwise))
}

#[test]
fn blocked_sample_06_matches_references() {
    // The public API blocks sample 06 for its 80 % upward share (see
    // tests/ugr_table.rs), but its table is still held to the references.
    let model = load_sample(6);
    assert!(model.ugr_table().is_err());
    let table = unchecked(&model);
    let python = read_reference("ugr_table_06_python.csv");
    let relux = read_reference("ugr_table_06_Relux.csv");
    for (row, (python, relux)) in python.iter().zip(&relux).enumerate() {
        for (column, (python, relux)) in python.iter().zip(relux).enumerate() {
            let actual = computed(&table, 6, row, column);
            let Reference::Value(python) = *python else {
                panic!("python reference has no bounds");
            };
            assert!(
                (actual - python).abs() <= PYTHON_TOLERANCE,
                "[{row}][{column}]: {actual} vs python {python}"
            );
            let rounded = round_to_tenth(actual);
            let deviation = match *relux {
                Reference::Value(expected) => (rounded - expected).abs(),
                Reference::Below(bound) => (rounded - bound).max(0.0),
            };
            assert!(
                deviation <= RELUX_TOLERANCE,
                "[{row}][{column}]: {actual} vs Relux {relux:?}"
            );
        }
    }
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
fn upper_zones_without_data_have_no_flux() {
    // Sample 11 emits 2.58 cd/klm at 90° and nothing above. Cut at 90°, the
    // file must keep its flux fractions and UGR table. eulumdat-ugr would
    // continue the 90° intensity into the 95°..175° zones instead.
    let full = load_sample(11);
    let lower = truncated_at_90(&full);
    assert_eq!(FluxFractions::new(&lower), FluxFractions::new(&full));
    assert_eq!(unchecked(&lower), unchecked(&full));

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
    assert!(all_cells(&unchecked(&lower)).all(|cell| cell.is_some()));
}

fn unchecked(ldt: &Eulumdat) -> UgrTable {
    ldt.ugr_table_with_spacing(CATALOGUE_SPACING_TO_HEIGHT)
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
    let table = unchecked(&model);
    assert_eq!(table.lamp_flux(), 0.0);
    assert!(all_cells(&table).all(|cell| cell.is_none()));

    let table = unchecked(&Eulumdat::default());
    assert!(all_cells(&table).all(|cell| cell.is_none()));
}

#[test]
fn constants_follow_cie_190_order() {
    assert_eq!(UGR_ROOMS[0], UgrRoom { x_h: 2, y_h: 2 });
    assert_eq!(UGR_ROOMS[10], UgrRoom { x_h: 4, y_h: 8 });
    assert_eq!(UGR_ROOMS[18], UgrRoom { x_h: 12, y_h: 8 });
    let reflectances = |ceiling, walls, floor| UgrReflectances {
        ceiling,
        walls,
        floor,
    };
    assert_eq!(UGR_REFLECTANCES[0], reflectances(0.7, 0.5, 0.2));
    assert_eq!(UGR_REFLECTANCES[4], reflectances(0.3, 0.3, 0.2));
    for (room, &UgrRoom { x_h: x, y_h: y }) in UGR_ROOMS.iter().enumerate() {
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
            std::hint::black_box(model.ugr_table().expect("sample is not blocked"));
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
