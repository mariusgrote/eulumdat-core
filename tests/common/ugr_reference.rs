// UGR fixture helpers shared by the crate-internal tests in `src/ugr/` and the
// integration tests. They only use `std`, so both can include this file with
// `#[path]`.

#![allow(dead_code)]

use std::fmt;
use std::fs;
use std::path::PathBuf;

pub const SAMPLE_COUNT: usize = 11;

pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ugr")
}

pub fn read_sample_text(number: usize) -> String {
    let path = fixture_dir().join(format!("sample_{number:02}.ldt"));
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()))
}

/// Parsed reference cell: an exact value, or Relux's "<10.0" upper bound.
#[derive(Debug, Clone, Copy)]
pub enum Reference {
    Value(f64),
    Below(f64),
}

/// Reads a 19 × 10 reference table from `tests/fixtures/ugr/reference`.
pub fn read_reference(name: &str) -> Vec<Vec<Reference>> {
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

pub fn round_to_tenth(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

/// Largest deviation of a table from a reference, with the position.
#[derive(Default, Clone, Copy)]
pub struct Deviation {
    pub max: f64,
    pub number: usize,
    pub row: usize,
    pub column: usize,
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
    pub fn record(&mut self, deviation: f64, number: usize, row: usize, column: usize) {
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
