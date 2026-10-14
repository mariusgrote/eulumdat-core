#![allow(clippy::pedantic)]
#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
mod common;

use eulumdat_core::{Eulumdat, Symmetry, TypeIndicator, ValidationSettings};

#[test]
fn synthetic_absolute_data_round_trips_without_changing_flux_or_intensities() {
    let mut original =
        common::synthetic_model(Symmetry::Rotational, TypeIndicator::PointSourceWithSymmetry);
    original.lamps[0].lamp_count = -3;
    original.lamps[0].total_luminous_flux = 2500.0;
    let (model, warnings) = Eulumdat::parse_bytes(original.to_text().as_bytes()).unwrap();
    assert!(model.is_absolute_photometry());
    assert_eq!(model, original);
    assert!(!warnings.iter().any(|w| w.field == "Number of lamps"));
    let (reloaded, _) = Eulumdat::parse(&model.to_text()).unwrap();
    assert_eq!(reloaded, original);
}

#[test]
fn signed_counts_preserve_magnitude_and_relative_counts_remain_valid() {
    for count in [-1000, -42, -1, 1, 42, 1000] {
        let mut model =
            common::synthetic_model(Symmetry::Rotational, TypeIndicator::PointSourceWithSymmetry);
        model.lamps[0].lamp_count = count;
        let (reloaded, warnings) = Eulumdat::parse(&model.to_text()).unwrap();
        assert_eq!(reloaded, model);
        assert_eq!(reloaded.is_absolute_photometry(), count < 0);
        assert!(!warnings.iter().any(|w| w.field == "Number of lamps"));
    }
}

#[test]
fn invalid_count_magnitudes_are_reported_without_integer_overflow() {
    for count in [0, 1001, i32::MAX] {
        let mut model =
            common::synthetic_model(Symmetry::Rotational, TypeIndicator::PointSourceWithSymmetry);
        model.lamps[0].lamp_count = count;
        let warnings = model.validate(ValidationSettings::unrestricted()).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| w.field == "Number of lamps" && w.lamp_index == Some(0))
        );
    }
    for count in [-1001, i32::MIN] {
        let mut model =
            common::synthetic_model(Symmetry::Rotational, TypeIndicator::PointSourceWithSymmetry);
        model.lamps[0].lamp_count = count;
        assert!(model.validate(ValidationSettings::unrestricted()).is_err());
        assert!(Eulumdat::parse(&model.to_text()).is_err());
    }
    for count in ["2147483648", "-2147483649", "9223372036854775808"] {
        let text =
            common::synthetic_ldt(Symmetry::Rotational, TypeIndicator::PointSourceWithSymmetry);
        let mut lines: Vec<_> = text.lines().collect();
        lines[26] = count;
        assert!(Eulumdat::parse(&lines.join("\n")).is_err());
    }
}

#[test]
fn absolute_mode_rejects_multiple_and_mixed_lamp_sets() {
    for second_count in [-2, 2] {
        let mut model =
            common::synthetic_model(Symmetry::Rotational, TypeIndicator::PointSourceWithSymmetry);
        model.lamps[0].lamp_count = -1;
        let mut second = model.lamps[0].clone();
        second.lamp_count = second_count;
        model.lamps.push(second);
        assert!(model.validate(ValidationSettings::unrestricted()).is_err());
        assert!(Eulumdat::parse(&model.to_text()).is_err());
        model.lamps.reverse();
        assert!(model.validate(ValidationSettings::unrestricted()).is_err());
    }
}

#[cfg(feature = "generate-svg")]
#[test]
fn absolute_report_identifies_emitters_and_luminaire_flux() {
    let mut model =
        common::synthetic_model(Symmetry::Rotational, TypeIndicator::PointSourceWithSymmetry);
    model.lamps[0].lamp_count = -1;
    let report = model
        .to_report_svg(&eulumdat_core::ReportOptions::default())
        .unwrap();
    assert!(report.contains("Absolute photometry (luminaire flux)"));
    assert!(report.contains("1 emitters (absolute)"));
    assert!(!report.contains("-1 x"));
}

#[test]
fn absolute_flux_warning_uses_luminaire_label_and_lamp_index() {
    let mut model =
        common::synthetic_model(Symmetry::Rotational, TypeIndicator::PointSourceWithSymmetry);
    model.lamps[0].lamp_count = -1;
    model.lamps[0].total_luminous_flux = 0.0;
    let warnings = model.validate(ValidationSettings::unrestricted()).unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.field == "Luminous flux of luminaire" && w.lamp_index == Some(0))
    );
    model.lamps[0].lamp_count = 1;
    let warnings = model.validate(ValidationSettings::unrestricted()).unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.field == "Total luminous flux of lamps" && w.lamp_index == Some(0))
    );
}
