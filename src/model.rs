use crate::EulumdatError;

#[derive(Debug, Clone, PartialEq)]
/// Parsed EULUMDAT luminaire data.
///
/// Field names follow the EULUMDAT record layout. Numeric lengths are in
/// millimeters, angular values are in degrees, flux fractions and output ratios
/// are percentages, and luminous intensities are candela per kilolumen values as
/// stored in the file.
pub struct Eulumdat {
    /// Free-form file identification line.
    pub identification: String,
    /// Luminaire type indicator.
    pub type_indicator: TypeIndicator,
    /// Photometric distribution symmetry indicator.
    pub symmetry: Symmetry,
    /// Declared angular step between C-planes, in degrees.
    pub c_plane_step: f64,
    /// Declared angular step between gamma angles, in degrees.
    pub gamma_step: f64,
    /// Measurement report number.
    pub measurement_report_number: String,
    /// Luminaire name.
    pub luminaire_name: String,
    /// Luminaire number or catalogue identifier.
    pub luminaire_number: String,
    /// Source file name recorded in the EULUMDAT data.
    pub file_name: String,
    /// Date and user field from the EULUMDAT file.
    pub date_user: String,
    /// Length or diameter of the luminaire.
    pub luminaire_length: f64,
    /// Width of the luminaire.
    pub luminaire_width: f64,
    /// Height of the luminaire.
    pub luminaire_height: f64,
    /// Length or diameter of the luminous area.
    pub luminous_area_length: f64,
    /// Width of the luminous area.
    pub luminous_area_width: f64,
    /// Height of the luminous area in the C0 plane.
    pub luminous_area_height_c0: f64,
    /// Height of the luminous area in the C90 plane.
    pub luminous_area_height_c90: f64,
    /// Height of the luminous area in the C180 plane.
    pub luminous_area_height_c180: f64,
    /// Height of the luminous area in the C270 plane.
    pub luminous_area_height_c270: f64,
    /// Downward flux fraction recorded in the file, as a percentage.
    pub downward_flux_fraction: f64,
    /// Light output ratio recorded in the file, as a percentage.
    pub light_output_ratio: f64,
    /// Conversion factor for luminous intensities.
    pub conversion_factor: f64,
    /// Luminaire tilt during measurement, in degrees.
    pub tilt: f64,
    /// Lamp sets described by the file.
    pub lamps: Vec<LampSet>,
    /// Direct-ratio values `k1` through `k10`.
    pub direct_ratios: [f64; 10],
    /// Expanded C-plane angles, in degrees.
    pub c_planes: Vec<f64>,
    /// Gamma angles, in degrees.
    pub gamma_angles: Vec<f64>,
    /// Stored luminous intensity rows indexed by C-plane, then gamma angle.
    pub intensities: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
/// One standard set of lamps described by an EULUMDAT file.
pub struct LampSet {
    /// Number of lamps in the set.
    pub lamp_count: u32,
    /// Lamp type description.
    pub lamp_type: String,
    /// Total luminous flux of the lamps in lumens.
    pub total_luminous_flux: f64,
    /// Lamp color temperature description.
    pub color_temperature: String,
    /// Lamp color rendering index description.
    pub color_rendering_index: String,
    /// Lamp wattage including ballast.
    pub wattage_including_ballast: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// EULUMDAT symmetry indicator for the luminous intensity distribution.
pub enum Symmetry {
    /// No symmetry; all declared C-plane rows are stored.
    None,
    /// Rotational symmetry; one C-plane row is stored.
    Rotational,
    /// Symmetry between the C0 and C180 planes.
    C0C180,
    /// Symmetry between the C90 and C270 planes.
    C90C270,
    /// Symmetry across both C0/C180 and C90/C270 axes.
    C0C180AndC90C270,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// EULUMDAT type indicator.
pub enum TypeIndicator {
    /// Point source with symmetry, serialized as raw value `1`.
    PointSourceWithSymmetry,
    /// Linear luminaire, serialized as raw value `2`.
    LinearLuminaire,
    /// Point source without symmetry, serialized as raw value `3`.
    PointSourceWithoutSymmetry,
}

#[derive(Debug, Clone, PartialEq)]
/// Replacement photometric distribution for an [`Eulumdat`] model.
pub struct Distribution {
    /// Symmetry of the replacement distribution.
    pub symmetry: Symmetry,
    /// Angular step between C-planes, in degrees.
    pub c_plane_step: f64,
    /// Angular step between gamma angles, in degrees.
    pub gamma_step: f64,
    /// Expanded C-plane angles, in degrees.
    pub c_planes: Vec<f64>,
    /// Gamma angles, in degrees.
    pub gamma_angles: Vec<f64>,
    /// Stored luminous intensity rows indexed by C-plane, then gamma angle.
    pub intensities: Vec<Vec<f64>>,
}

/// String-length validation settings.
///
/// `max_identification_len` is also used for the general line fields from the
/// reference implementation: measurement report number, luminaire name,
/// luminaire number, and date/user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationSettings {
    /// Maximum length for identification and other general text fields.
    pub max_identification_len: Option<usize>,
    /// Maximum length for the file name field.
    pub max_file_name_len: Option<usize>,
    /// Maximum length for each lamp type field.
    pub max_lamp_type_len: Option<usize>,
    /// Maximum length for each color temperature field.
    pub max_color_temperature_len: Option<usize>,
    /// Maximum length for each color rendering index field.
    pub max_color_rendering_index_len: Option<usize>,
}

impl Default for Eulumdat {
    fn default() -> Self {
        Self {
            identification: String::new(),
            type_indicator: TypeIndicator::PointSourceWithSymmetry,
            symmetry: Symmetry::None,
            c_plane_step: 0.0,
            gamma_step: 0.0,
            measurement_report_number: String::new(),
            luminaire_name: String::new(),
            luminaire_number: String::new(),
            file_name: String::new(),
            date_user: String::new(),
            luminaire_length: 0.0,
            luminaire_width: 0.0,
            luminaire_height: 0.0,
            luminous_area_length: 0.0,
            luminous_area_width: 0.0,
            luminous_area_height_c0: 0.0,
            luminous_area_height_c90: 0.0,
            luminous_area_height_c180: 0.0,
            luminous_area_height_c270: 0.0,
            downward_flux_fraction: 0.0,
            light_output_ratio: 0.0,
            conversion_factor: 0.0,
            tilt: 0.0,
            lamps: Vec::new(),
            direct_ratios: [0.0; 10],
            c_planes: Vec::new(),
            gamma_angles: Vec::new(),
            intensities: Vec::new(),
        }
    }
}

impl Eulumdat {
    #[must_use]
    /// Returns the number of expanded C-plane angles.
    pub fn c_plane_count(&self) -> usize {
        self.c_planes.len()
    }

    #[must_use]
    /// Returns the number of gamma angles in each intensity row.
    pub fn gamma_count(&self) -> usize {
        self.gamma_angles.len()
    }

    /// Returns the number of C-plane rows stored for the current symmetry.
    pub fn stored_c_plane_count(&self) -> Result<usize, EulumdatError> {
        self.symmetry.stored_c_plane_count(self.c_plane_count())
    }

    /// Replaces the photometric distribution after validating its matrix shape.
    pub fn replace_distribution(
        &mut self,
        distribution: Distribution,
    ) -> Result<(), EulumdatError> {
        validate_distribution_shape(
            distribution.symmetry,
            &distribution.c_planes,
            &distribution.gamma_angles,
            &distribution.intensities,
        )?;

        self.symmetry = distribution.symmetry;
        self.c_plane_step = distribution.c_plane_step;
        self.gamma_step = distribution.gamma_step;
        self.c_planes = distribution.c_planes;
        self.gamma_angles = distribution.gamma_angles;
        self.intensities = distribution.intensities;
        Ok(())
    }
}

pub(crate) fn validate_distribution_shape(
    symmetry: Symmetry,
    c_planes: &[f64],
    gamma_angles: &[f64],
    intensities: &[Vec<f64>],
) -> Result<(), EulumdatError> {
    if c_planes.is_empty() {
        return Err(EulumdatError::DistributionShape(
            "C-plane list must not be empty".to_string(),
        ));
    }
    if gamma_angles.is_empty() {
        return Err(EulumdatError::DistributionShape(
            "gamma angle list must not be empty".to_string(),
        ));
    }
    let expected = symmetry.stored_c_plane_count(c_planes.len())?;
    if intensities.len() != expected {
        return Err(EulumdatError::DistributionShape(format!(
            "expected {expected} stored C-plane rows, got {}",
            intensities.len()
        )));
    }
    for (index, row) in intensities.iter().enumerate() {
        if row.len() != gamma_angles.len() {
            return Err(EulumdatError::DistributionShape(format!(
                "intensity row {index} has {} values, expected {}",
                row.len(),
                gamma_angles.len()
            )));
        }
    }
    Ok(())
}

impl Symmetry {
    /// Converts a raw EULUMDAT symmetry indicator into a typed value.
    pub fn from_raw(value: u8) -> Result<Self, EulumdatError> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Rotational),
            2 => Ok(Self::C0C180),
            3 => Ok(Self::C90C270),
            4 => Ok(Self::C0C180AndC90C270),
            _ => Err(EulumdatError::Validation(format!(
                "Symmetry indicator = {value} -> value is out of range"
            ))),
        }
    }

    #[must_use]
    /// Returns the raw EULUMDAT symmetry indicator value.
    pub fn raw_value(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Rotational => 1,
            Self::C0C180 => 2,
            Self::C90C270 => 3,
            Self::C0C180AndC90C270 => 4,
        }
    }

    /// Returns the number of intensity rows stored for this symmetry.
    pub fn stored_c_plane_count(
        self,
        declared_c_plane_count: usize,
    ) -> Result<usize, EulumdatError> {
        Ok(match self {
            Self::None => declared_c_plane_count,
            Self::Rotational => 1,
            Self::C0C180 | Self::C90C270 => declared_c_plane_count / 2 + 1,
            Self::C0C180AndC90C270 => declared_c_plane_count / 4 + 1,
        })
    }
}

impl TypeIndicator {
    /// Converts a raw EULUMDAT type indicator into a typed value.
    pub fn from_raw(value: u8) -> Result<Self, EulumdatError> {
        match value {
            1 => Ok(Self::PointSourceWithSymmetry),
            2 => Ok(Self::LinearLuminaire),
            3 => Ok(Self::PointSourceWithoutSymmetry),
            _ => Err(EulumdatError::Validation(format!(
                "Type indicator = {value} -> value is out of range"
            ))),
        }
    }

    #[must_use]
    /// Returns the raw EULUMDAT type indicator value.
    pub fn raw_value(self) -> u8 {
        match self {
            Self::PointSourceWithSymmetry => 1,
            Self::LinearLuminaire => 2,
            Self::PointSourceWithoutSymmetry => 3,
        }
    }
}

impl ValidationSettings {
    #[must_use]
    /// Returns settings that do not enforce text length limits.
    pub const fn unrestricted() -> Self {
        Self {
            max_identification_len: None,
            max_file_name_len: None,
            max_lamp_type_len: None,
            max_color_temperature_len: None,
            max_color_rendering_index_len: None,
        }
    }

    #[must_use]
    /// Returns the legacy EULUMDAT text length limits used by strict tools.
    pub const fn restricted() -> Self {
        Self {
            max_identification_len: Some(78),
            max_file_name_len: Some(8),
            max_lamp_type_len: Some(24),
            max_color_temperature_len: Some(16),
            max_color_rendering_index_len: Some(6),
        }
    }
}
