use crate::{
    Eulumdat, EulumdatError, Symmetry, ValidationSettings, ValidationWarning,
    model::validate_distribution_shape,
};

impl Eulumdat {
    /// Validates structural constraints and returns non-fatal warnings.
    pub fn validate(
        &self,
        settings: ValidationSettings,
    ) -> Result<Vec<ValidationWarning>, EulumdatError> {
        let mut warnings = Vec::new();

        validate_distribution_shape(
            self.symmetry,
            &self.c_planes,
            &self.gamma_angles,
            &self.intensities,
        )?;

        if self.c_plane_count() > 721 {
            return validation_error(format!(
                "Number of C-planes = {} -> value is out of range",
                self.c_plane_count()
            ));
        }
        if !(0.0..=360.0).contains(&self.c_plane_step) {
            return validation_error(format!(
                "Distance between C-planes = {} -> value is out of range",
                self.c_plane_step
            ));
        }
        if self.gamma_count() > 361 {
            return validation_error(format!(
                "Number of luminous intensities in each C-plane = {} -> value is out of range",
                self.gamma_count()
            ));
        }
        if !(0.0..=180.0).contains(&self.gamma_step) {
            return validation_error(format!(
                "Distance between luminous intensities per C-plane = {} -> value is out of range",
                self.gamma_step
            ));
        }
        if !(1..=20).contains(&self.lamps.len()) {
            return validation_error(format!(
                "Number of standard sets of lamps = {} -> value is out of range",
                self.lamps.len()
            ));
        }

        warn_len(
            &mut warnings,
            "Identification",
            self.identification.len(),
            settings.max_identification_len,
        );
        warn_len(
            &mut warnings,
            "Measurement report number",
            self.measurement_report_number.len(),
            settings.max_identification_len,
        );
        warn_len(
            &mut warnings,
            "Luminaire name",
            self.luminaire_name.len(),
            settings.max_identification_len,
        );
        warn_len(
            &mut warnings,
            "Luminaire number",
            self.luminaire_number.len(),
            settings.max_identification_len,
        );
        warn_len(
            &mut warnings,
            "File name",
            self.file_name.len(),
            settings.max_file_name_len,
        );
        warn_len(
            &mut warnings,
            "Date/user",
            self.date_user.len(),
            settings.max_identification_len,
        );

        warn_range(
            &mut warnings,
            "Length/diameter of luminaire",
            self.luminaire_length,
            1.0,
            9999.0,
        );
        warn_range(
            &mut warnings,
            "Width of luminaire",
            self.luminaire_width,
            0.0,
            9999.0,
        );
        warn_range(
            &mut warnings,
            "Height of luminaire",
            self.luminaire_height,
            0.0,
            9999.0,
        );
        warn_range(
            &mut warnings,
            "Length/diameter of luminous area",
            self.luminous_area_length,
            0.0,
            9999.0,
        );
        warn_range(
            &mut warnings,
            "Width of luminous area",
            self.luminous_area_width,
            0.0,
            9999.0,
        );
        warn_range(
            &mut warnings,
            "Height of luminous area C0-plane",
            self.luminous_area_height_c0,
            0.0,
            9999.0,
        );
        warn_range(
            &mut warnings,
            "Height of luminous area C90-plane",
            self.luminous_area_height_c90,
            0.0,
            9999.0,
        );
        warn_range(
            &mut warnings,
            "Height of luminous area C180-plane",
            self.luminous_area_height_c180,
            0.0,
            9999.0,
        );
        warn_range(
            &mut warnings,
            "Height of luminous area C270-plane",
            self.luminous_area_height_c270,
            0.0,
            9999.0,
        );
        warn_range(
            &mut warnings,
            "Downward flux fraction",
            self.downward_flux_fraction,
            0.0,
            100.0,
        );
        warn_range(
            &mut warnings,
            "Light output ratio of luminaire",
            self.light_output_ratio,
            0.0,
            100.0,
        );
        warn_range(
            &mut warnings,
            "Conversion factor for luminous intensities",
            self.conversion_factor,
            0.0,
            10.0,
        );
        warn_range(
            &mut warnings,
            "Tilt of luminaire during measurement",
            self.tilt,
            -180.0,
            180.0,
        );

        for lamp in &self.lamps {
            if !(1..=1000).contains(&lamp.lamp_count) {
                warnings.push(warning(
                    "Number of lamps",
                    format!(
                        "Number of lamps = {} -> value is out of range",
                        lamp.lamp_count
                    ),
                ));
            }
            warn_len(
                &mut warnings,
                "Type of lamps",
                lamp.lamp_type.len(),
                settings.max_lamp_type_len,
            );
            warn_range(
                &mut warnings,
                "Total luminous flux of lamps",
                lamp.total_luminous_flux,
                1.0,
                9_999_999.0,
            );
            warn_len(
                &mut warnings,
                "Color temperature of lamps",
                lamp.color_temperature.len(),
                settings.max_color_temperature_len,
            );
            warn_len(
                &mut warnings,
                "Color rendering index",
                lamp.color_rendering_index.len(),
                settings.max_color_rendering_index_len,
            );
            warn_range(
                &mut warnings,
                "Wattage including ballast",
                lamp.wattage_including_ballast,
                0.1,
                10_000.0,
            );
        }

        for (index, value) in self.direct_ratios.iter().enumerate() {
            if !(0.0..=10.0).contains(value) {
                warnings.push(warning(
                    format!("k[{}]", index + 1),
                    format!("k[{}] = {} -> value is out of range", index + 1, value),
                ));
            }
        }

        let mut all_under_one = true;
        let mut avg = 0.0;
        let mut count = 0usize;
        for row in &self.intensities {
            for value in row {
                if !(0.0..=1_000_000.0).contains(value) {
                    return validation_error(format!(
                        "Luminous intensity distribution = {value} -> values are out of range"
                    ));
                }
                avg += value;
                count += 1;
                if *value >= 1.0 {
                    all_under_one = false;
                }
            }
        }
        if all_under_one {
            if count > 0 {
                avg /= count as f64;
            }
            return validation_error(format!(
                "Distribution of luminous intensity (avg = {avg}) -> values are too low"
            ));
        }

        ensure_strictly_increasing(&self.c_planes, "C-planes not sorted")?;
        ensure_strictly_increasing(&self.gamma_angles, "Gamma-planes not sorted")?;
        if self.gamma_angles.first().copied().unwrap_or(0.0) < 0.0
            || self.gamma_angles.last().copied().unwrap_or(0.0) > 180.0
        {
            return validation_error("Wrong Gamma-planes scheme");
        }

        validate_c_plane_scheme(self)?;
        Ok(warnings)
    }
}

fn validate_c_plane_scheme(ldt: &Eulumdat) -> Result<(), EulumdatError> {
    let stored = ldt.stored_c_plane_count()?;
    match ldt.symmetry {
        Symmetry::None if stored >= 4 => {
            require_plane(ldt, 90.0, "Missing C90-plane")?;
            require_plane(ldt, 180.0, "Missing C180-plane")?;
            require_plane(ldt, 270.0, "Missing C270-plane")
        }
        Symmetry::None => validation_error("Wrong C-planes scheme"),
        Symmetry::Rotational if stored == 1 => Ok(()),
        Symmetry::Rotational => validation_error("Wrong C-planes scheme"),
        Symmetry::C0C180 if stored >= 3 => require_plane(ldt, 90.0, "Missing C90-plane"),
        Symmetry::C0C180 => validation_error("Wrong C-planes scheme"),
        Symmetry::C90C270 if stored >= 3 => require_plane(ldt, 180.0, "Missing C180-plane"),
        Symmetry::C90C270 => validation_error("Wrong C-planes scheme"),
        Symmetry::C0C180AndC90C270 if stored >= 2 => Ok(()),
        Symmetry::C0C180AndC90C270 => validation_error("Wrong C-planes scheme"),
    }
}

fn require_plane(ldt: &Eulumdat, plane: f64, message: &str) -> Result<(), EulumdatError> {
    if ldt.c_planes.contains(&plane) {
        Ok(())
    } else {
        validation_error(message)
    }
}

fn ensure_strictly_increasing(values: &[f64], message: &str) -> Result<(), EulumdatError> {
    if values.windows(2).all(|pair| pair[0] < pair[1]) {
        Ok(())
    } else {
        validation_error(message)
    }
}

fn warn_len(
    warnings: &mut Vec<ValidationWarning>,
    field: impl Into<String>,
    len: usize,
    max: Option<usize>,
) {
    let field = field.into();
    if let Some(max) = max
        && len > max
    {
        warnings.push(warning(
            field.clone(),
            format!("{field} = {len} -> line is too long"),
        ));
    }
}

fn warn_range(warnings: &mut Vec<ValidationWarning>, field: &str, value: f64, min: f64, max: f64) {
    if !(min..=max).contains(&value) {
        warnings.push(warning(
            field,
            format!("{field} = {value} -> value is out of range"),
        ));
    }
}

fn warning(field: impl Into<String>, message: String) -> ValidationWarning {
    ValidationWarning {
        field: field.into(),
        message,
    }
}

fn validation_error<T>(message: impl Into<String>) -> Result<T, EulumdatError> {
    Err(EulumdatError::Validation(message.into()))
}
