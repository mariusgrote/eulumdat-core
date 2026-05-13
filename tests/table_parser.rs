#![allow(clippy::pedantic)]
#![allow(unused_crate_dependencies)]
mod common;

use eulumdat_core::{Symmetry, parse_table_text};

#[test]
fn table_parser_accepts_symmetry_shapes() {
    let cases = [
        (vec![0.0], Symmetry::Rotational, vec![0.0], 1),
        (
            vec![0.0, 90.0, 180.0, 270.0, 360.0],
            Symmetry::None,
            vec![0.0, 90.0, 180.0, 270.0, 360.0],
            5,
        ),
        (
            vec![0.0, 90.0, 180.0],
            Symmetry::C0C180,
            vec![0.0, 90.0, 180.0, 270.0],
            3,
        ),
        (
            vec![90.0, 180.0, 270.0],
            Symmetry::C90C270,
            vec![0.0, 90.0, 180.0, 270.0],
            3,
        ),
        (
            vec![0.0, 45.0, 90.0],
            Symmetry::C0C180AndC90C270,
            vec![0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0],
            3,
        ),
    ];

    for (input, symmetry, expanded, stored_count) in cases {
        let parsed = parse_table_text(&common::table_text(&input)).expect("table should parse");
        assert_eq!(parsed.symmetry, symmetry);
        assert_eq!(parsed.c_planes, expanded);
        assert_eq!(parsed.gamma_angles, vec![0.0, 45.0, 90.0, 135.0, 180.0]);
        assert_eq!(parsed.intensities.len(), stored_count);
    }
}

#[test]
fn table_parser_rejects_invalid_tables() {
    let cases = [
        (
            "\t0\n0\tbad\n45\t1\n90\t1\n135\t1\n180\t1\n",
            "Data are not real numbers",
        ),
        (
            "\t90\t0\n0\t10\t11\n45\t12\t13\n90\t14\t15\n135\t16\t17\n180\t18\t19\n",
            "C-planes not sorted",
        ),
        ("\t0\n0\t10\n", "Lack of useful data"),
        (
            &common::table_text(&[0.0, 180.0, 270.0, 360.0]),
            "Missing C90-plane",
        ),
        (
            &common::table_text(&[10.0, 20.0, 30.0]),
            "Wrong C-planes scheme",
        ),
        (
            "\t0\n0\t0.5\n45\t0.5\n90\t0.5\n135\t0.5\n180\t0.5\n",
            "Values are too low",
        ),
    ];

    for (text, expected) in cases {
        let error = parse_table_text(text).unwrap_err().to_string();
        assert!(error.contains(expected), "{error}");
    }
}
