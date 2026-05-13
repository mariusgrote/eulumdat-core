use std::fmt::Write as _;
use std::path::Path;

use crate::{Eulumdat, EulumdatError, TypeIndicator, ValidationSettings};

impl Eulumdat {
    #[must_use]
    /// Serializes the model to EULUMDAT text.
    pub fn to_text(&self) -> String {
        self.to_text_checked()
            .expect("valid Eulumdat shape is required for serialization")
    }

    #[must_use]
    /// Serializes the model to UTF-8 EULUMDAT bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_text().into_bytes()
    }

    /// Validates and writes the model as EULUMDAT text to a path.
    pub fn write_path(&self, path: impl AsRef<Path>) -> Result<(), EulumdatError> {
        self.validate(ValidationSettings::unrestricted())?;
        std::fs::write(path, self.to_text_checked()?)?;
        Ok(())
    }

    pub(crate) fn to_text_checked(&self) -> Result<String, EulumdatError> {
        let stored_c_count = self.symmetry.stored_c_plane_count(self.c_plane_count())?;
        if self.intensities.len() < stored_c_count {
            return Err(EulumdatError::DistributionShape(format!(
                "expected at least {stored_c_count} stored C-plane rows, got {}",
                self.intensities.len()
            )));
        }

        let mut out = String::new();
        push_line(&mut out, &self.identification);
        push_line(&mut out, &self.serialized_type_indicator().to_string());
        push_line(&mut out, &self.symmetry.raw_value().to_string());
        push_line(&mut out, &self.c_plane_count().to_string());
        push_line(&mut out, &format_number(self.c_plane_step));
        push_line(&mut out, &self.gamma_count().to_string());
        push_line(&mut out, &format_number(self.gamma_step));
        push_line(&mut out, &self.measurement_report_number);
        push_line(&mut out, &self.luminaire_name);
        push_line(&mut out, &self.luminaire_number);
        push_line(&mut out, &self.file_name);
        push_line(&mut out, &self.date_user);
        push_line(&mut out, &format_number(self.luminaire_length));
        push_line(&mut out, &format_number(self.luminaire_width));
        push_line(&mut out, &format_number(self.luminaire_height));
        push_line(&mut out, &format_number(self.luminous_area_length));
        push_line(&mut out, &format_number(self.luminous_area_width));
        push_line(&mut out, &format_number(self.luminous_area_height_c0));
        push_line(&mut out, &format_number(self.luminous_area_height_c90));
        push_line(&mut out, &format_number(self.luminous_area_height_c180));
        push_line(&mut out, &format_number(self.luminous_area_height_c270));
        push_line(&mut out, &format_number(self.downward_flux_fraction));
        push_line(&mut out, &format_number(self.light_output_ratio));
        push_line(&mut out, &format_number(self.conversion_factor));
        push_line(&mut out, &format_number(self.tilt));
        push_line(&mut out, &self.lamps.len().to_string());
        for lamp in &self.lamps {
            push_line(&mut out, &lamp.lamp_count.to_string());
            push_line(&mut out, &lamp.lamp_type);
            push_line(&mut out, &format_number(lamp.total_luminous_flux));
            push_line(&mut out, &lamp.color_temperature);
            push_line(&mut out, &lamp.color_rendering_index);
            push_line(&mut out, &format_number(lamp.wattage_including_ballast));
        }
        for value in self.direct_ratios {
            push_line(&mut out, &format_number(value));
        }
        for value in &self.c_planes {
            push_line(&mut out, &format_number(*value));
        }
        for value in &self.gamma_angles {
            push_line(&mut out, &format_number(*value));
        }
        for row in self.intensities.iter().take(stored_c_count) {
            for value in row.iter().take(self.gamma_count()) {
                push_line(&mut out, &format_number(*value));
            }
        }
        Ok(out)
    }

    fn serialized_type_indicator(&self) -> u8 {
        if self.type_indicator != TypeIndicator::LinearLuminaire {
            if self.symmetry == crate::Symmetry::Rotational {
                1
            } else {
                3
            }
        } else {
            2
        }
    }
}

fn push_line(out: &mut String, value: &str) {
    let _ = writeln!(out, "{value}");
}

fn format_number(value: f64) -> String {
    value.to_string()
}
