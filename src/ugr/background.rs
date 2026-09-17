use std::f64::consts::PI;

use super::tables::{F_GL, F_T, ROOM_K};
use crate::Eulumdat;

/// Width of the zones used for the zonal flux, in degrees.
const ZONE_WIDTH_DEG: f64 = 10.0;
/// Number of 10° zones over the full sphere; the first half points downwards.
const ZONE_COUNT: usize = 18;
const DOWNWARD_ZONE_COUNT: usize = ZONE_COUNT / 2;

/// Room-independent flux fractions of a luminaire for the background luminance.
///
/// Port of the first half of eulumdat-ugr `BackgroundLuminance.compute`
/// (CIE 190:2010 §4.2). All fluxes are in lm per 1000 lm lamp flux.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FluxFractions {
    /// Cumulative downward fluxes `Φ_zL1` to `Φ_zL4`.
    cumulative: [f64; 4],
    /// Downward light output ratio `R_DLO`.
    downward_ratio: f64,
    /// Upward light output ratio `R_ULO`.
    upward_ratio: f64,
}

impl FluxFractions {
    /// Computes the zonal fluxes from the intensity distribution.
    ///
    /// The intensity at each 10° zone midpoint (5°, 15°, …, 175°) is
    /// interpolated along γ in every expanded C-plane and averaged over the
    /// planes. Zone flux is `I · 2π · sin(γ_mid) · Δγ`, and the zones are scaled
    /// so that they sum to the file's light output ratio.
    ///
    /// Deliberate deviation from eulumdat-ugr: a zone midpoint outside the
    /// measured γ range has zero flux. Python's `np.interp` holds the last
    /// measured value instead, so a file covering only γ ≤ 90° gets upward
    /// flux from its 90° intensities there. That flux lowers `R_DLO`, adds
    /// `R_ULO`, and changes the background luminance.
    ///
    /// Returns `None` if the distribution has no C-planes or γ angles, or no
    /// flux at all.
    pub(crate) fn new(ldt: &Eulumdat) -> Option<Self> {
        let (&first_gamma, &last_gamma) = (ldt.gamma_angles.first()?, ldt.gamma_angles.last()?);
        if ldt.c_planes.is_empty() {
            return None;
        }

        let zone_width = ZONE_WIDTH_DEG.to_radians();
        let mut zones = [0.0; ZONE_COUNT];
        for (index, zone) in zones.iter_mut().enumerate() {
            let midpoint = (index as f64 + 0.5) * ZONE_WIDTH_DEG;
            if !(first_gamma..=last_gamma).contains(&midpoint) {
                continue;
            }
            let mut sum = 0.0;
            for &c_plane in &ldt.c_planes {
                sum += ldt.intensity_at(c_plane, midpoint)?;
            }
            let mean = sum / ldt.c_planes.len() as f64;
            *zone = mean * 2.0 * PI * midpoint.to_radians().sin() * zone_width;
        }

        let total: f64 = zones.iter().sum();
        if total <= 0.0 {
            return None;
        }
        let scale = ldt.light_output_ratio / 100.0 / (total / 1000.0);
        for zone in &mut zones {
            *zone *= scale;
        }

        let (down, up) = zones.split_at(DOWNWARD_ZONE_COUNT);
        let sum_to = |end: usize| down[..end].iter().sum::<f64>();
        // 0.130 and 0.547 are the parts of the 40–50° and 70–80° zones that
        // belong to Φ_zL1 and Φ_zL3 (CIE 190:2010 §4.2).
        let cumulative = [
            sum_to(4) + 0.130 * down[4],
            sum_to(6),
            sum_to(7) + 0.547 * down[7],
            sum_to(DOWNWARD_ZONE_COUNT),
        ];
        Some(Self {
            cumulative,
            downward_ratio: sum_to(DOWNWARD_ZONE_COUNT) / 1000.0,
            upward_ratio: up.iter().sum::<f64>() / 1000.0,
        })
    }

    /// Background luminance `L_b` in cd/m² for each reflectance of
    /// [`UGR_REFLECTANCES`](super::UGR_REFLECTANCES).
    ///
    /// `room` indexes [`UGR_ROOMS`](super::UGR_ROOMS). `wall_flux_density` is
    /// `B = Φ · N / A_w` in lm/m², with Φ the lamp flux of one luminaire.
    /// Following CIE 190:2010 eq. (7)–(12):
    ///
    /// - `F_DF = Σ Φ_zLi · F_GLi / 1000`
    /// - `F_DW = R_DLO − F_DF`, `F_DC = R_ULO`
    /// - `F_UWID = F_DF · F_T.FW + F_DW · F_T.WW−1 + F_DC · F_T.CW`
    /// - `E_WID = B · F_UWID` and `L_b = E_WID / π`
    pub(crate) fn background_luminance(&self, room: usize, wall_flux_density: f64) -> [f64; 5] {
        let (_, f_gl) = F_GL
            .iter()
            .find(|(k, _)| *k == ROOM_K[room])
            .expect("every CIE 190 room index k has F_GL factors");
        let direct_floor = self
            .cumulative
            .iter()
            .zip(f_gl)
            .map(|(flux, factor)| flux * factor)
            .sum::<f64>()
            / 1000.0;
        let direct_walls = self.downward_ratio - direct_floor;
        let direct_ceiling = self.upward_ratio;

        F_T[room].map(|[floor, walls, ceiling]| {
            let utilance = direct_floor * floor + direct_walls * walls + direct_ceiling * ceiling;
            wall_flux_density * utilance / PI
        })
    }
}
