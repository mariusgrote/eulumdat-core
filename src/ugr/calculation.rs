use super::UGR_ROOMS;
use super::background::FluxFractions;
use super::geometry::{CATALOGUE_SPACING_TO_HEIGHT, MOUNTING_HEIGHT, Orientation, RoomGrid};
use super::guth::position_index;
use crate::Eulumdat;

/// Number of reflectance combinations per orientation.
const REFLECTANCE_COUNT: usize = 5;

/// UGR table of a luminaire after CIE 190:2010 for the standard rooms.
#[derive(Debug, Clone, PartialEq)]
pub struct UgrTable {
    /// UGR values, one row per room of [`UGR_ROOMS`] in CIE 190 order.
    ///
    /// Columns 0–4 are crosswise and 5–9 endwise, each in the reflectance
    /// order of [`UGR_REFLECTANCES`](super::UGR_REFLECTANCES). Values are not
    /// rounded; `None` marks cells that cannot be computed.
    pub values: [[Option<f64>; 10]; 19],
    /// Lamp flux in lm that `values` refer to.
    pub lamp_flux: f64,
}

impl Eulumdat {
    /// Computes the UGR table with the CIE 117 tabular method.
    ///
    /// Port of eulumdat-ugr `UgrCalculator.compute` with luminaire spacing
    /// S = 0.25·H, the catalogue value. For each room and orientation,
    /// Σ L²·ω/p² over all luminaires is computed once; every reflectance then
    /// yields `UGR = 8·log10(0.25 / L_b · Σ)`.
    ///
    /// The values refer to the real flux of the first lamp set. eulumdat-ugr
    /// multiplies that flux by the lamp count, although EULUMDAT already
    /// stores the total flux of the set; both agree for single-lamp sets.
    /// Cells are `None` if no luminaire contributes or `L_b` is not positive.
    pub(crate) fn ugr_table(&self) -> UgrTable {
        self.ugr_table_with_spacing(CATALOGUE_SPACING_TO_HEIGHT)
    }

    /// Computes the UGR table with luminaire spacing S = `spacing_to_height`·H.
    ///
    /// CIE 190:2010 publishes its validation example for S = 1.0·H.
    pub(crate) fn ugr_table_with_spacing(&self, spacing_to_height: f64) -> UgrTable {
        let lamp_flux = self
            .lamps
            .first()
            .map_or(0.0, |lamps| lamps.total_luminous_flux);
        let mut values = [[None; 10]; 19];
        let Some(fractions) = FluxFractions::new(self) else {
            return UgrTable { values, lamp_flux };
        };

        for (room, (row, &(x_h, y_h))) in values.iter_mut().zip(&UGR_ROOMS).enumerate() {
            let grid = RoomGrid::new(x_h, y_h, spacing_to_height);
            let wall_flux_density = lamp_flux * grid.luminaire_count() as f64 / grid.wall_area();
            let background = fractions.background_luminance(room, wall_flux_density);

            for (orientation, cells) in [Orientation::Crosswise, Orientation::Endwise]
                .into_iter()
                .zip(row.chunks_exact_mut(REFLECTANCE_COUNT))
            {
                let Some(glare_sum) = self.glare_sum(&grid, orientation) else {
                    continue;
                };
                for (cell, luminance) in cells.iter_mut().zip(background) {
                    if luminance > 0.0 {
                        *cell = Some(8.0 * (0.25 / luminance * glare_sum).log10());
                    }
                }
            }
        }
        UgrTable { values, lamp_flux }
    }

    /// Σ L²·ω/p² over all luminaires in the room; `None` if none contributes.
    ///
    /// Luminaires with a NaN position index, no positive luminance, or no
    /// positive solid angle are skipped, as in eulumdat-ugr.
    fn glare_sum(&self, grid: &RoomGrid, orientation: Orientation) -> Option<f64> {
        let mut sum = 0.0;
        let mut contributes = false;
        for luminaire in grid.half_luminaires(orientation) {
            let index = position_index(MOUNTING_HEIGHT / luminaire.r, luminaire.t / luminaire.r);
            if index.is_nan() {
                continue;
            }
            let Some(luminance) = self.luminance_at(luminaire.c_deg, luminaire.gamma_deg) else {
                continue;
            };
            let solid_angle = self.projected_area(luminaire.c_deg, luminaire.gamma_deg)
                / luminaire.distance_squared;
            if luminance > 0.0 && solid_angle > 0.0 {
                // The position stands for both luminaires at ±T.
                sum += 2.0 * luminance * luminance * solid_angle / (index * index);
                contributes = true;
            }
        }
        contributes.then_some(sum)
    }
}
