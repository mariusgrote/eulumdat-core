use crate::EulumdatError;

#[derive(Debug, Clone, PartialEq)]
pub struct Eulumdat {
    pub identification: String,
    pub type_indicator: TypeIndicator,
    pub symmetry: Symmetry,
    pub c_plane_step: f64,
    pub gamma_step: f64,
    pub measurement_report_number: String,
    pub luminaire_name: String,
    pub luminaire_number: String,
    pub file_name: String,
    pub date_user: String,
    pub luminaire_length: f64,
    pub luminaire_width: f64,
    pub luminaire_height: f64,
    pub luminous_area_length: f64,
    pub luminous_area_width: f64,
    pub luminous_area_height_c0: f64,
    pub luminous_area_height_c90: f64,
    pub luminous_area_height_c180: f64,
    pub luminous_area_height_c270: f64,
    pub downward_flux_fraction: f64,
    pub light_output_ratio: f64,
    pub conversion_factor: f64,
    pub tilt: f64,
    pub lamps: Vec<LampSet>,
    pub direct_ratios: [f64; 10],
    pub c_planes: Vec<f64>,
    pub gamma_angles: Vec<f64>,
    pub intensities: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LampSet {
    pub lamp_count: u32,
    pub lamp_type: String,
    pub total_luminous_flux: f64,
    pub color_temperature: String,
    pub color_rendering_index: String,
    pub wattage_including_ballast: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symmetry {
    None,
    Rotational,
    C0C180,
    C90C270,
    C0C180AndC90C270,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeIndicator {
    PointSourceWithSymmetry,
    LinearLuminaire,
    PointSourceWithoutSymmetry,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Distribution {
    pub symmetry: Symmetry,
    pub c_plane_step: f64,
    pub gamma_step: f64,
    pub c_planes: Vec<f64>,
    pub gamma_angles: Vec<f64>,
    pub intensities: Vec<Vec<f64>>,
}

/// String-length validation settings.
///
/// `max_identification_len` is also used for the general line fields from the
/// reference implementation: measurement report number, luminaire name,
/// luminaire number, and date/user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationSettings {
    pub max_identification_len: Option<usize>,
    pub max_file_name_len: Option<usize>,
    pub max_lamp_type_len: Option<usize>,
    pub max_color_temperature_len: Option<usize>,
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
    pub fn c_plane_count(&self) -> usize {
        self.c_planes.len()
    }

    #[must_use]
    pub fn gamma_count(&self) -> usize {
        self.gamma_angles.len()
    }

    pub fn stored_c_plane_count(&self) -> Result<usize, EulumdatError> {
        self.symmetry.stored_c_plane_count(self.c_plane_count())
    }

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
    pub fn raw_value(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Rotational => 1,
            Self::C0C180 => 2,
            Self::C90C270 => 3,
            Self::C0C180AndC90C270 => 4,
        }
    }

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
