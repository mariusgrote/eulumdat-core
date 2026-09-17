#![allow(clippy::pedantic)]
#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]

// Luminance and projected-area parity with eulumdat-luminance is covered by the
// crate-internal unit tests in src/ugr/tests.rs; these tests only need the
// public API.

use std::fs;
use std::path::PathBuf;

use eulumdat_core::Eulumdat;

const SAMPLE_COUNT: usize = 11;
const UGR_ROWS: usize = 19;
const UGR_COLUMNS: usize = 10;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ugr")
}

fn read_ugr_table(name: &str) -> Vec<Vec<String>> {
    let path = fixture_dir().join("reference").join(name);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split(',')
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect()
}

fn assert_ugr_table_shape(name: &str, table: &[Vec<String>]) {
    assert_eq!(table.len(), UGR_ROWS, "{name}: row count");
    for (index, row) in table.iter().enumerate() {
        assert_eq!(
            row.len(),
            UGR_COLUMNS,
            "{name}: column count in row {index}"
        );
    }
}

#[test]
fn all_ugr_samples_parse() {
    for number in 1..=SAMPLE_COUNT {
        let path = fixture_dir().join(format!("sample_{number:02}.ldt"));
        let text = fs::read_to_string(&path).expect("sample fixture should be readable");
        let (ldt, _warnings) = Eulumdat::parse(&text)
            .unwrap_or_else(|error| panic!("{} should parse: {error}", path.display()));
        assert!(!ldt.lamps.is_empty(), "sample {number:02} has a lamp set");
        assert!(
            ldt.lamps[0].total_luminous_flux > 0.0,
            "sample {number:02} has lamp flux"
        );
        assert!(ldt.total_output() > 0.0, "sample {number:02} emits light");
    }
}

#[test]
fn external_reference_tables_have_ugr_shape() {
    let mut names = Vec::new();
    for number in 1..=SAMPLE_COUNT {
        names.push(format!("ugr_table_{number:02}_Relux.csv"));
        names.push(format!("ugr_table_{number:02}_Dialux.csv"));
    }
    names.push("ugr_table_11_cie190.csv".to_string());

    for name in names {
        let table = read_ugr_table(&name);
        assert_ugr_table_shape(&name, &table);
        for cell in table.iter().flatten() {
            // Relux writes "<10.0" for values below 10.
            let number = cell.strip_prefix('<').unwrap_or(cell);
            let value: f64 = number
                .parse()
                .unwrap_or_else(|_| panic!("{name}: invalid cell {cell:?}"));
            assert!(
                (5.0..=40.0).contains(&value),
                "{name}: {value} out of range"
            );
        }
    }
}

#[test]
fn python_reference_tables_have_ugr_shape() {
    for number in 1..=SAMPLE_COUNT {
        let name = format!("ugr_table_{number:02}_python.csv");
        let table = read_ugr_table(&name);
        assert_ugr_table_shape(&name, &table);
        for cell in table.iter().flatten() {
            let (_, decimals) = cell
                .split_once('.')
                .unwrap_or_else(|| panic!("{name}: cell {cell:?} has no decimals"));
            assert_eq!(decimals.len(), 4, "{name}: cell {cell:?}");
            let value: f64 = cell.parse().expect("python reference cell is a number");
            assert!(value.is_finite(), "{name}: cell {cell:?}");
        }
    }
}
