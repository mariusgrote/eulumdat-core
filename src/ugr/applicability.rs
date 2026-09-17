use std::fmt::{Display, Formatter};

use crate::model::validate_distribution_shape;
use crate::{Eulumdat, Symmetry};

/// Largest allowed gap between neighbouring gamma angles, in degrees
/// (DIN EN 13032-2).
const MAX_GAMMA_STEP_DEG: f64 = 5.0;
/// Largest allowed gap between neighbouring C-planes, in degrees
/// (DIN EN 13032-2).
const MAX_C_STEP_DEG: f64 = 15.0;
/// Largest upward share of the luminaire flux for the tabular method
/// (`LiTG` Publ. 20).
const MAX_UPWARD_FRACTION: f64 = 0.65;
/// Largest intensity difference between mirrored C-planes, relative to the
/// peak intensity.
///
/// The tabular method assumes symmetry to the C0–C180 and C90–C270 planes and
/// only evaluates the quadrant C 0–90°. Files stored without symmetry often
/// describe symmetric luminaires with a few percent of measurement noise, so
/// an exact comparison would block them, while asymmetric optics such as wall
/// washers deviate far more. The value is a judgement call, not taken from a
/// standard; the reference samples all store symmetric data and do not
/// calibrate it.
const MAX_ASYMMETRY: f64 = 0.05;
/// C-plane step of the numerical symmetry check, in degrees.
const SYMMETRY_C_STEP_DEG: f64 = 5.0;
/// Tolerance for comparing stored angles with limits, in degrees.
const ANGLE_TOLERANCE_DEG: f64 = 1e-6;

/// Reason why the UGR tabular method does not apply to a luminaire.
///
/// Returned by [`Eulumdat::ugr_table`]. Fractions are between 0 and 1.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum UgrBlocker {
    /// A luminous-area dimension is negative or non-finite, or the bottom area
    /// is not positive and finite, so luminance is undefined.
    NoLuminousArea,
    /// There is no lamp set, or the first one has no positive finite flux.
    NoLampFlux,
    /// The light output ratio is not positive and finite, so the background
    /// luminance is undefined.
    NoLightOutputRatio,
    /// The intensity conversion factor is not positive and finite.
    InvalidConversionFactor,
    /// The intensity matrix does not match the angles, the angles are not
    /// finite, ascending, and in range, an intensity is negative or non-finite,
    /// or no intensity is positive.
    InvalidDistribution,
    /// The stored gamma angles do not cover 0° to 90°.
    IncompleteDistribution {
        /// Smallest stored gamma angle; `None` without gamma angles.
        min_gamma: Option<f64>,
        /// Largest stored gamma angle; `None` without gamma angles.
        max_gamma: Option<f64>,
    },
    /// The largest gap between stored angles exceeds 5° for gamma or 15° for
    /// C (DIN EN 13032-2).
    AngleGridTooCoarse {
        /// Largest gap between C-planes, including the wrap from the last
        /// plane to 360°; `None` for rotational symmetry.
        c_step: Option<f64>,
        /// Largest gap between gamma angles.
        gamma_step: f64,
    },
    /// More than 65 % of the luminaire flux is emitted upwards (`LiTG` Publ. 20).
    IndirectShareTooHigh {
        /// Upward share of the flux calculated from the distribution.
        upward_fraction: f64,
    },
    /// The distribution is not symmetric to the C0–C180 and C90–C270 planes.
    Asymmetric {
        /// Largest intensity difference between mirrored directions with
        /// gamma ≤ 90°, relative to the peak intensity.
        max_deviation: f64,
    },
}

impl Display for UgrBlocker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLuminousArea => write!(
                f,
                "Luminous-area dimensions -> must be non-negative and finite with positive area"
            ),
            Self::NoLampFlux => write!(
                f,
                "First lamp set -> must exist with positive finite luminous flux"
            ),
            Self::NoLightOutputRatio => {
                write!(f, "Light output ratio -> must be positive and finite")
            }
            Self::InvalidConversionFactor => write!(
                f,
                "Intensity conversion factor -> must be positive and finite"
            ),
            Self::InvalidDistribution => write!(
                f,
                "Luminous intensity distribution -> must be finite, non-negative, and contain light"
            ),
            Self::IncompleteDistribution {
                min_gamma: Some(min),
                max_gamma: Some(max),
            } => write!(f, "Gamma angles = {min}°..{max}° -> must cover 0°..90°"),
            Self::IncompleteDistribution { .. } => {
                write!(f, "Gamma angles missing -> must cover 0°..90°")
            }
            Self::AngleGridTooCoarse { c_step, gamma_step } => {
                write!(f, "Angle steps = gamma {gamma_step}°")?;
                if let Some(c_step) = c_step {
                    write!(f, ", C {c_step}°")?;
                }
                write!(
                    f,
                    " -> too coarse (max gamma {MAX_GAMMA_STEP_DEG}°, C {MAX_C_STEP_DEG}°)"
                )
            }
            Self::IndirectShareTooHigh { upward_fraction } => write!(
                f,
                "Upward flux fraction = {:.1} % -> above {} %",
                upward_fraction * 100.0,
                MAX_UPWARD_FRACTION * 100.0
            ),
            Self::Asymmetric { max_deviation } => write!(
                f,
                "Asymmetry = {:.1} % of peak intensity -> above {} %",
                max_deviation * 100.0,
                MAX_ASYMMETRY * 100.0
            ),
        }
    }
}

impl Eulumdat {
    /// Collects every reason why the UGR tabular method does not apply.
    pub(crate) fn ugr_blockers(&self) -> Vec<UgrBlocker> {
        let mut blockers = Vec::new();
        if !self.has_valid_luminous_geometry() {
            blockers.push(UgrBlocker::NoLuminousArea);
        }
        if !self
            .lamps
            .first()
            .is_some_and(|lamps| is_positive_finite(lamps.total_luminous_flux))
        {
            blockers.push(UgrBlocker::NoLampFlux);
        }
        if !is_positive_finite(self.light_output_ratio) {
            blockers.push(UgrBlocker::NoLightOutputRatio);
        }
        if !is_positive_finite(self.conversion_factor) {
            blockers.push(UgrBlocker::InvalidConversionFactor);
        }

        let Some(peak) = self.valid_distribution_peak() else {
            blockers.push(UgrBlocker::InvalidDistribution);
            return blockers;
        };
        let (min_gamma, max_gamma) = (
            self.gamma_angles.first().copied(),
            self.gamma_angles.last().copied(),
        );
        if !(min_gamma.is_some_and(|gamma| gamma <= ANGLE_TOLERANCE_DEG)
            && max_gamma.is_some_and(|gamma| gamma >= 90.0 - ANGLE_TOLERANCE_DEG))
        {
            blockers.push(UgrBlocker::IncompleteDistribution {
                min_gamma,
                max_gamma,
            });
        }
        if let Some(blocker) = self.coarse_grid() {
            blockers.push(blocker);
        }
        let downward_fraction = self.calculated_downward_flux_fraction();
        if !downward_fraction.is_finite() {
            blockers.push(UgrBlocker::InvalidDistribution);
            return blockers;
        }
        let upward_fraction = 1.0 - downward_fraction / 100.0;
        if upward_fraction > MAX_UPWARD_FRACTION {
            blockers.push(UgrBlocker::IndirectShareTooHigh { upward_fraction });
        }
        let max_deviation = self.max_mirror_deviation() / peak;
        if max_deviation > MAX_ASYMMETRY {
            blockers.push(UgrBlocker::Asymmetric { max_deviation });
        }
        blockers
    }

    /// Peak intensity if the distribution has a consistent shape, ascending
    /// angles, and positive finite intensities; `None` otherwise.
    fn valid_distribution_peak(&self) -> Option<f64> {
        validate_distribution_shape(
            self.symmetry,
            &self.c_planes,
            &self.gamma_angles,
            &self.intensities,
        )
        .ok()?;
        if !valid_angles(&self.c_planes, 360.0, false)
            || !valid_angles(&self.gamma_angles, 180.0, true)
        {
            return None;
        }
        let peak = self
            .intensities
            .iter()
            .flatten()
            .copied()
            .try_fold(0.0_f64, |peak, value| {
                (value.is_finite() && value >= 0.0).then(|| peak.max(value))
            })?;
        is_positive_finite(peak).then_some(peak)
    }

    /// Whether every luminous-area value used by the UGR calculation has a
    /// physically meaningful sign and finite value.
    fn has_valid_luminous_geometry(&self) -> bool {
        if !is_positive_finite(self.luminous_area_length)
            || !is_non_negative_finite(self.luminous_area_width)
            || !is_non_negative_finite(self.luminous_area_height_c0)
        {
            return false;
        }
        if self.luminous_area_width > 0.0
            && ![
                self.luminous_area_height_c90,
                self.luminous_area_height_c180,
                self.luminous_area_height_c270,
            ]
            .into_iter()
            .all(is_non_negative_finite)
        {
            return false;
        }
        is_positive_finite(self.projected_area(0.0, 0.0))
    }

    /// `AngleGridTooCoarse` if a gamma or C gap exceeds its limit.
    fn coarse_grid(&self) -> Option<UgrBlocker> {
        let gamma_step = max_gap(&self.gamma_angles);
        let c_step = (self.symmetry != Symmetry::Rotational).then(|| {
            let wrap = self
                .c_planes
                .first()
                .zip(self.c_planes.last())
                .map_or(0.0, |(first, last)| first + 360.0 - last);
            max_gap(&self.c_planes).max(wrap)
        });
        let too_coarse = gamma_step > MAX_GAMMA_STEP_DEG + ANGLE_TOLERANCE_DEG
            || c_step.is_some_and(|step| step > MAX_C_STEP_DEG + ANGLE_TOLERANCE_DEG);
        too_coarse.then_some(UgrBlocker::AngleGridTooCoarse { c_step, gamma_step })
    }

    /// Largest |I(C) − I(C')| with C' ∈ {180° − C, 180° + C, 360° − C} over
    /// C = 0°, 5°, …, 90° and all stored gamma ≤ 90°.
    fn max_mirror_deviation(&self) -> f64 {
        let steps = (90.0 / SYMMETRY_C_STEP_DEG).round() as u32;
        let mut max = 0.0_f64;
        for &gamma in &self.gamma_angles {
            if gamma > 90.0 + ANGLE_TOLERANCE_DEG {
                break;
            }
            for step in 0..=steps {
                let c = f64::from(step) * SYMMETRY_C_STEP_DEG;
                let Some(intensity) = self.intensity_at(c, gamma) else {
                    continue;
                };
                for mirrored in [180.0 - c, 180.0 + c, 360.0 - c] {
                    if let Some(other) = self.intensity_at(mirrored, gamma) {
                        max = max.max((intensity - other).abs());
                    }
                }
            }
        }
        max
    }
}

/// `true` for positive finite numbers.
fn is_positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn is_non_negative_finite(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

/// Whether an angle axis is finite, in `[0, upper]` (or `[0, upper)`), and
/// strictly ascending.
fn valid_angles(values: &[f64], upper: f64, inclusive_upper: bool) -> bool {
    values.iter().all(|&value| {
        value.is_finite()
            && value >= 0.0
            && if inclusive_upper {
                value <= upper
            } else {
                value < upper
            }
    }) && values.windows(2).all(|pair| pair[0] < pair[1])
}

/// Largest difference between neighbouring values; 0 for fewer than two.
fn max_gap(values: &[f64]) -> f64 {
    values
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .fold(0.0, f64::max)
}
