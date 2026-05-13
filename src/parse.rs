use std::path::Path;

use crate::{
    Eulumdat, EulumdatError, LampSet, ParseContext, Symmetry, TextEncoding, TypeIndicator,
    ValidationSettings, ValidationWarning, encoding::decode_ldt_bytes,
};

struct LineReader<'a> {
    input: &'a str,
    offset: usize,
    line_number: usize,
}

impl<'a> LineReader<'a> {
    const fn new(input: &'a str) -> Self {
        Self {
            input,
            offset: 0,
            line_number: 1,
        }
    }

    fn read_line(&mut self, field: &str) -> Result<String, EulumdatError> {
        if self.offset >= self.input.len() {
            return Err(self.parse_error(field, None, "unexpected end of file"));
        }

        let start = self.offset;
        let remaining = &self.input[start..];
        let (line, consumed) = if let Some(newline) = remaining.find('\n') {
            (&remaining[..newline], newline + 1)
        } else {
            (remaining, remaining.len())
        };
        self.offset += consumed;
        self.line_number += 1;
        Ok(line.strip_suffix('\r').unwrap_or(line).to_string())
    }

    fn parse_error(&self, field: &str, raw_line: Option<String>, reason: &str) -> EulumdatError {
        EulumdatError::Parse(ParseContext {
            field: field.to_string(),
            line_number: self.line_number,
            byte_offset: self.offset,
            raw_line,
            reason: reason.to_string(),
        })
    }

    fn parse_i64(&mut self, field: &str) -> Result<i64, EulumdatError> {
        let raw = self.read_line(field)?;
        if raw.is_empty() {
            return Ok(0);
        }
        raw.parse::<i64>()
            .map_err(|_| self.parse_error(field, Some(raw), "invalid integer"))
    }

    fn parse_f64(&mut self, field: &str) -> Result<f64, EulumdatError> {
        let raw = self.read_line(field)?;
        let normalized = raw.replace(['_', ' '], "").replace(',', ".");
        if normalized.is_empty() {
            return Ok(0.0);
        }
        normalized
            .parse::<f64>()
            .map_err(|_| self.parse_error(field, Some(raw), "invalid number"))
    }
}

impl Eulumdat {
    /// Parses EULUMDAT text using unrestricted validation settings.
    pub fn parse(input: &str) -> Result<(Self, Vec<ValidationWarning>), EulumdatError> {
        Self::parse_with_settings(input, ValidationSettings::unrestricted())
    }

    /// Parses EULUMDAT text using the supplied validation settings.
    pub fn parse_with_settings(
        input: &str,
        settings: ValidationSettings,
    ) -> Result<(Self, Vec<ValidationWarning>), EulumdatError> {
        let mut reader = LineReader::new(input);
        let mut loaded = Self {
            identification: reader.read_line("identification")?,
            ..Self::default()
        };

        let type_indicator_raw = reader.parse_i64("type indicator")?;
        if !(1..=3).contains(&type_indicator_raw) {
            return Err(EulumdatError::Validation(format!(
                "Type indicator = {type_indicator_raw} -> value is out of range"
            )));
        }
        loaded.type_indicator = TypeIndicator::from_raw(type_indicator_raw as u8)?;

        let symmetry_raw = reader.parse_i64("symmetry indicator")?;
        if !(0..=4).contains(&symmetry_raw) {
            return Err(EulumdatError::Validation(format!(
                "Symmetry indicator = {symmetry_raw} -> value is out of range"
            )));
        }
        loaded.symmetry = Symmetry::from_raw(symmetry_raw as u8)?;

        let c_count = parse_non_negative_count(&mut reader, "number of C-planes")?;
        loaded.c_plane_step = reader.parse_f64("distance between C-planes")?;
        let gamma_count = parse_non_negative_count(
            &mut reader,
            "number of luminous intensities in each C-plane",
        )?;
        loaded.gamma_step =
            reader.parse_f64("distance between luminous intensities per C-plane")?;
        loaded.measurement_report_number = reader.read_line("measurement report number")?;
        loaded.luminaire_name = reader.read_line("luminaire name")?;
        loaded.luminaire_number = reader.read_line("luminaire number")?;
        loaded.file_name = reader.read_line("file name")?;
        loaded.date_user = reader.read_line("date/user")?;
        loaded.luminaire_length = reader.parse_f64("length/diameter of luminaire")?;
        loaded.luminaire_width = reader.parse_f64("width of luminaire")?;
        loaded.luminaire_height = reader.parse_f64("height of luminaire")?;
        loaded.luminous_area_length = reader.parse_f64("length/diameter of luminous area")?;
        loaded.luminous_area_width = reader.parse_f64("width of luminous area")?;
        loaded.luminous_area_height_c0 = reader.parse_f64("height of luminous area C0-plane")?;
        loaded.luminous_area_height_c90 = reader.parse_f64("height of luminous area C90-plane")?;
        loaded.luminous_area_height_c180 =
            reader.parse_f64("height of luminous area C180-plane")?;
        loaded.luminous_area_height_c270 =
            reader.parse_f64("height of luminous area C270-plane")?;
        loaded.downward_flux_fraction = reader.parse_f64("downward flux fraction")?;
        loaded.light_output_ratio = reader.parse_f64("light output ratio of luminaire")?;
        loaded.conversion_factor =
            reader.parse_f64("conversion factor for luminous intensities")?;
        loaded.tilt = reader.parse_f64("tilt of luminaire during measurement")?;

        let lamp_count = parse_non_negative_count(&mut reader, "number of standard sets of lamps")?;
        loaded.lamps.reserve(lamp_count);
        for index in 0..lamp_count {
            let prefix = format!("lamp[{index}].");
            let lamp_count_raw = reader.parse_i64(&(prefix.clone() + "number of lamps"))?;
            if lamp_count_raw < 0 {
                return Err(EulumdatError::Validation(format!(
                    "Number of lamps = {lamp_count_raw} -> value is out of range"
                )));
            }
            loaded.lamps.push(LampSet {
                lamp_count: u32::try_from(lamp_count_raw).map_err(|_| {
                    EulumdatError::Validation(format!(
                        "Number of lamps = {lamp_count_raw} -> value is out of range"
                    ))
                })?,
                lamp_type: reader.read_line(&(prefix.clone() + "type of lamps"))?,
                total_luminous_flux: reader
                    .parse_f64(&(prefix.clone() + "total luminous flux of lamps"))?,
                color_temperature: reader
                    .read_line(&(prefix.clone() + "color temperature of lamps"))?,
                color_rendering_index: reader
                    .read_line(&(prefix.clone() + "color rendering index"))?,
                wattage_including_ballast: reader
                    .parse_f64(&(prefix + "wattage including ballast"))?,
            });
        }

        for index in 0..10 {
            loaded.direct_ratios[index] = reader.parse_f64(&format!("directRatio[{index}]"))?;
        }

        loaded.c_planes.reserve(c_count);
        for index in 0..c_count {
            loaded
                .c_planes
                .push(reader.parse_f64(&format!("cPlane[{index}]"))?);
        }
        loaded.gamma_angles.reserve(gamma_count);
        for index in 0..gamma_count {
            loaded
                .gamma_angles
                .push(reader.parse_f64(&format!("gamma[{index}]"))?);
        }

        let stored_c_count = loaded.symmetry.stored_c_plane_count(c_count)?;
        loaded.intensities.reserve(stored_c_count);
        for c in 0..stored_c_count {
            let mut row = Vec::with_capacity(gamma_count);
            for g in 0..gamma_count {
                row.push(reader.parse_f64(&format!("luminousIntensity[c={c},g={g}]"))?);
            }
            loaded.intensities.push(row);
        }

        let warnings = loaded.validate(settings)?;
        Ok((loaded, warnings))
    }

    /// Parses EULUMDAT bytes as UTF-8 or Windows-1252 using unrestricted settings.
    pub fn parse_bytes(input: &[u8]) -> Result<(Self, Vec<ValidationWarning>), EulumdatError> {
        Self::parse_bytes_with_settings(input, ValidationSettings::unrestricted())
    }

    /// Parses EULUMDAT bytes and returns the detected text encoding.
    pub fn parse_bytes_detect_encoding(
        input: &[u8],
    ) -> Result<(Self, Vec<ValidationWarning>, TextEncoding), EulumdatError> {
        Self::parse_bytes_detect_encoding_with_settings(input, ValidationSettings::unrestricted())
    }

    /// Parses EULUMDAT bytes using the supplied validation settings.
    pub fn parse_bytes_with_settings(
        input: &[u8],
        settings: ValidationSettings,
    ) -> Result<(Self, Vec<ValidationWarning>), EulumdatError> {
        let (ldt, warnings, _) = Self::parse_bytes_detect_encoding_with_settings(input, settings)?;
        Ok((ldt, warnings))
    }

    /// Parses EULUMDAT bytes with settings and returns the detected text encoding.
    pub fn parse_bytes_detect_encoding_with_settings(
        input: &[u8],
        settings: ValidationSettings,
    ) -> Result<(Self, Vec<ValidationWarning>, TextEncoding), EulumdatError> {
        let (text, encoding) = decode_ldt_bytes(input);
        let (ldt, warnings) = Self::parse_with_settings(&text, settings)?;
        Ok((ldt, warnings, encoding))
    }

    /// Reads and parses an EULUMDAT file from a path.
    pub fn from_path(
        path: impl AsRef<Path>,
    ) -> Result<(Self, Vec<ValidationWarning>), EulumdatError> {
        let bytes = std::fs::read(path)?;
        Self::parse_bytes(&bytes)
    }
}

fn parse_non_negative_count(
    reader: &mut LineReader<'_>,
    field: &str,
) -> Result<usize, EulumdatError> {
    let value = reader.parse_i64(field)?;
    if value < 0 {
        return Err(EulumdatError::Validation(format!(
            "{field} = {value} -> value is out of range"
        )));
    }
    usize::try_from(value).map_err(|_| {
        EulumdatError::Validation(format!("{field} = {value} -> value is out of range"))
    })
}
