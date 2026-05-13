#![allow(clippy::pedantic)]
mod common;

use eulumdat_core::{Eulumdat, Symmetry, TypeIndicator};

#[test]
fn loads_synthetic_symmetry_matrix() {
    for symmetry in [
        Symmetry::None,
        Symmetry::Rotational,
        Symmetry::C0C180,
        Symmetry::C90C270,
        Symmetry::C0C180AndC90C270,
    ] {
        let text = common::synthetic_ldt(symmetry, TypeIndicator::PointSourceWithSymmetry);
        let (ldt, _) = Eulumdat::parse(&text).expect("synthetic LDT should parse");
        assert!(!ldt.lamps.is_empty());
        assert!(ldt.gamma_count() >= 1);
        assert!(ldt.stored_c_plane_count().expect("stored count") >= 1);
    }
}

#[test]
fn save_reload_round_trip_synthetic() {
    let original = common::synthetic_model(Symmetry::C0C180, TypeIndicator::LinearLuminaire);
    let serialized = original.to_text();
    let (reloaded, _) = Eulumdat::parse(&serialized).expect("serialized text should parse");
    assert_eq!(reloaded, original);
}

#[test]
fn missing_file_returns_error() {
    let error = Eulumdat::from_path("/path/that/does/not/exist.ldt").unwrap_err();
    assert!(error.to_string().contains("I/O error"));
}

#[test]
fn malformed_number_returns_context() {
    let source = common::synthetic_ldt(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    let mut lines: Vec<String> = source.lines().map(ToOwned::to_owned).collect();
    lines[1] = "not-an-int".to_string();
    let error = Eulumdat::parse(&lines.join("\n")).unwrap_err().to_string();
    assert!(error.contains("type indicator"), "{error}");
    assert!(error.contains("not-an-int"), "{error}");

    let mut lines: Vec<String> = source.lines().map(ToOwned::to_owned).collect();
    lines[26] = "not-a-lamp-count".to_string();
    let error = Eulumdat::parse(&lines.join("\n")).unwrap_err().to_string();
    assert!(error.contains("lamp[0].number of lamps"), "{error}");
    assert!(error.contains("not-a-lamp-count"), "{error}");

    let lines: Vec<String> = source.lines().map(ToOwned::to_owned).collect();
    let c_count = lines[3].parse::<usize>().unwrap();
    let lamp_count = lines[25].parse::<usize>().unwrap();
    let gamma_start = 26 + lamp_count * 6 + 10 + c_count;
    let mut changed = lines.clone();
    changed[gamma_start] = "not-a-gamma".to_string();
    let error = Eulumdat::parse(&changed.join("\n"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("gamma[0]"), "{error}");
    assert!(error.contains("not-a-gamma"), "{error}");

    let error = Eulumdat::parse(&lines[..2].join("\n"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("symmetry indicator"), "{error}");
    assert!(error.contains("unexpected end of file"), "{error}");
}

#[test]
fn parse_accepts_decimal_comma() {
    let mut lines: Vec<String> =
        common::synthetic_ldt(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry)
            .lines()
            .map(ToOwned::to_owned)
            .collect();
    lines[4] = "2,5".to_string();
    let (ldt, _) = Eulumdat::parse(&lines.join("\n")).expect("decimal comma should parse");
    assert_eq!(ldt.c_plane_step, 2.5);
}

#[test]
fn parse_strips_numeric_underscores_and_spaces() {
    let mut lines: Vec<String> =
        common::synthetic_ldt(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry)
            .lines()
            .map(ToOwned::to_owned)
            .collect();
    lines[12] = "1_2 3".to_string();
    let (ldt, _) = Eulumdat::parse(&lines.join("\n")).expect("stripped number should parse");
    assert_eq!(ldt.luminaire_length, 123.0);
}

#[test]
fn serialization_preserves_type_indicator_rule() {
    let mut ldt =
        common::synthetic_model(Symmetry::Rotational, TypeIndicator::PointSourceWithSymmetry);
    assert_eq!(ldt.to_text().lines().nth(1), Some("1"));

    ldt = common::synthetic_model(Symmetry::C0C180, TypeIndicator::PointSourceWithSymmetry);
    assert_eq!(ldt.to_text().lines().nth(1), Some("3"));

    ldt.type_indicator = TypeIndicator::LinearLuminaire;
    assert_eq!(ldt.to_text().lines().nth(1), Some("2"));
}

#[test]
fn parse_bytes_rejects_invalid_utf8() {
    let error = Eulumdat::parse_bytes(&[0xff, 0xfe]).unwrap_err();
    assert!(error.to_string().contains("invalid UTF-8"));
}
