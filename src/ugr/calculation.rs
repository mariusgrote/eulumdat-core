use super::background::FluxFractions;
use super::geometry::{CATALOGUE_SPACING_TO_HEIGHT, MOUNTING_HEIGHT, RoomGrid};
use super::guth::position_index;
use super::result::REFLECTANCE_COUNT;
use super::{UGR_ROOMS, UgrBlocker, UgrTable, UgrView};
use crate::Eulumdat;

impl Eulumdat {
    /// Computes the UGR table with the CIE 117 tabular method.
    ///
    /// The tabular method only applies to luminaires that meet its
    /// assumptions. If the data does not, the result is `Err` with every
    /// [`UgrBlocker`] found, not only the first.
    ///
    /// Port of eulumdat-ugr `UgrCalculator.compute` with luminaire spacing
    /// S = 0.25·H, the catalogue value. For each room and view,
    /// Σ L²·ω/p² over all luminaires is computed once; every reflectance then
    /// yields `UGR = 8·log10(0.25 / L_b · Σ)`.
    ///
    /// Glare-source luminance uses the stored cd/klm intensities multiplied by
    /// [`Eulumdat::conversion_factor`] and by the total flux of the first lamp
    /// set in klm. Field 26c is already the total flux of that set, so the lamp
    /// count is not applied again. Background luminance uses that same lamp-set
    /// flux and the light output ratio. The returned table therefore has
    /// [`FluxBasis::LampFlux`](super::FluxBasis::LampFlux) as its native basis;
    /// callers can request a 1000 lm basis from [`UgrTable`] accessors.
    pub fn ugr_table(&self) -> Result<UgrTable, Vec<UgrBlocker>> {
        let blockers = self.ugr_blockers();
        if blockers.is_empty() {
            Ok(self.ugr_table_with_spacing(CATALOGUE_SPACING_TO_HEIGHT))
        } else {
            Err(blockers)
        }
    }

    /// Computes the UGR table with luminaire spacing S = `spacing_to_height`·H
    /// without checking whether the tabular method applies.
    ///
    /// CIE 190:2010 publishes its validation example for S = 1.0·H. Cells are
    /// `None` if no luminaire contributes or `L_b` is not positive.
    pub(crate) fn ugr_table_with_spacing(&self, spacing_to_height: f64) -> UgrTable {
        let lamp_flux = self
            .lamps
            .first()
            .map_or(0.0, |lamps| lamps.total_luminous_flux);
        let mut values = [[None; 2 * REFLECTANCE_COUNT]; UGR_ROOMS.len()];
        let Some(fractions) = FluxFractions::new(self) else {
            return UgrTable { values, lamp_flux };
        };

        for (room_index, (row, room)) in values.iter_mut().zip(&UGR_ROOMS).enumerate() {
            let grid = RoomGrid::new(room.x_h, room.y_h, spacing_to_height);
            let wall_flux_density = lamp_flux * grid.luminaire_count() as f64 / grid.wall_area();
            let background = fractions.background_luminance(room_index, wall_flux_density);

            let (view_cells, remainder) = row.as_chunks_mut::<REFLECTANCE_COUNT>();
            debug_assert!(remainder.is_empty());
            for (view, cells) in [UgrView::Crosswise, UgrView::Endwise]
                .into_iter()
                .zip(view_cells)
            {
                let Some(glare_sum) = self.glare_sum(&grid, view) else {
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
    fn glare_sum(&self, grid: &RoomGrid, view: UgrView) -> Option<f64> {
        let mut sum = 0.0;
        let mut contributes = false;
        for luminaire in grid.half_luminaires(view) {
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
