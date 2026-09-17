use super::{UGR_REFLECTANCES, UGR_ROOMS};

/// Number of reflectance combinations per view.
pub(crate) const REFLECTANCE_COUNT: usize = UGR_REFLECTANCES.len();
/// Index of the data sheet room 4H × 8H in [`UGR_ROOMS`].
const DATA_SHEET_ROOM: usize = 10;
/// Index of the data sheet reflectances 70/50/20 in [`UGR_REFLECTANCES`].
const DATA_SHEET_REFLECTANCE: usize = 0;

/// Standard room of the UGR table, as multiples of the mounting height H.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UgrRoom {
    /// Room width X across the line of sight, in multiples of H.
    pub x_h: u8,
    /// Room depth Y along the line of sight, in multiples of H.
    pub y_h: u8,
}

/// Reflectances of one UGR table column, as fractions from 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UgrReflectances {
    /// Ceiling cavity reflectance.
    pub ceiling: f64,
    /// Wall reflectance.
    pub walls: f64,
    /// Floor cavity (working plane) reflectance.
    pub floor: f64,
}

/// Viewing direction relative to the luminaire's C0–C180 axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UgrView {
    /// Luminaire axis across the line of sight.
    Crosswise,
    /// Luminaire axis along the line of sight.
    Endwise,
}

/// Lamp flux the UGR values refer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FluxBasis {
    /// Flux of the first lamp set, as stored in the file.
    LampFlux,
    /// A lamp flux of 1000 lm, as in CIE 190 catalogue tables.
    Normalized1000Lm,
}

/// UGR table of a luminaire after CIE 117:1995 and CIE 190:2010.
///
/// Holds 19 rooms of [`UGR_ROOMS`] × 2 views × 5 reflectances of
/// [`UGR_REFLECTANCES`]. Values are not rounded. A cell is `None` if no
/// luminaire in the room contributes glare, for example with a beam too narrow
/// to reach any observer position, or if the background luminance is not
/// positive.
#[derive(Debug, Clone, PartialEq)]
pub struct UgrTable {
    /// UGR values for the lamp flux, one row per room. Columns 0–4 are
    /// crosswise and 5–9 endwise, each in the order of [`UGR_REFLECTANCES`].
    pub(super) values: [[Option<f64>; 2 * REFLECTANCE_COUNT]; UGR_ROOMS.len()],
    /// Lamp flux in lm that `values` refer to.
    pub(super) lamp_flux: f64,
}

/// One room of a [`UgrTable`] for one [`FluxBasis`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UgrRow {
    /// The room of this row.
    pub room: UgrRoom,
    /// Crosswise values in the order of [`UGR_REFLECTANCES`].
    pub crosswise: [Option<f64>; REFLECTANCE_COUNT],
    /// Endwise values in the order of [`UGR_REFLECTANCES`].
    pub endwise: [Option<f64>; REFLECTANCE_COUNT],
}

impl UgrTable {
    #[must_use]
    /// Returns the lamp flux in lm that [`FluxBasis::LampFlux`] values refer to.
    pub fn lamp_flux(&self) -> f64 {
        self.lamp_flux
    }

    #[must_use]
    /// Returns `8·log10(Φ / 1000)`, the difference between
    /// [`FluxBasis::LampFlux`] and [`FluxBasis::Normalized1000Lm`] values.
    pub fn flux_correction(&self) -> f64 {
        8.0 * (self.lamp_flux / 1000.0).log10()
    }

    #[must_use]
    /// Returns the UGR value of one cell.
    ///
    /// `room` indexes [`UGR_ROOMS`] and `reflectance` indexes
    /// [`UGR_REFLECTANCES`]. Returns `None` for an index out of range or a
    /// cell without a value.
    pub fn value(
        &self,
        room: usize,
        view: UgrView,
        reflectance: usize,
        basis: FluxBasis,
    ) -> Option<f64> {
        if reflectance >= REFLECTANCE_COUNT {
            return None;
        }
        let offset = match view {
            UgrView::Crosswise => 0,
            UgrView::Endwise => REFLECTANCE_COUNT,
        };
        let value = self.values.get(room)?[offset + reflectance]?;
        Some(self.rebase(value, basis))
    }

    #[must_use]
    /// Returns the data sheet values `(crosswise, endwise)` for room 4H × 8H
    /// and reflectances 70/50/20.
    pub fn data_sheet_value(&self, basis: FluxBasis) -> (Option<f64>, Option<f64>) {
        let at = |view| self.value(DATA_SHEET_ROOM, view, DATA_SHEET_REFLECTANCE, basis);
        (at(UgrView::Crosswise), at(UgrView::Endwise))
    }

    /// Iterates over the rooms in the order of [`UGR_ROOMS`].
    pub fn rows(&self, basis: FluxBasis) -> impl Iterator<Item = UgrRow> + '_ {
        UGR_ROOMS
            .iter()
            .zip(&self.values)
            .map(move |(&room, values)| {
                let (crosswise, endwise) = values.split_at(REFLECTANCE_COUNT);
                let rebase = |cells: &[Option<f64>]| {
                    std::array::from_fn(|index| cells[index].map(|v| self.rebase(v, basis)))
                };
                UgrRow {
                    room,
                    crosswise: rebase(crosswise),
                    endwise: rebase(endwise),
                }
            })
    }

    fn rebase(&self, value: f64, basis: FluxBasis) -> f64 {
        match basis {
            FluxBasis::LampFlux => value,
            FluxBasis::Normalized1000Lm => value - self.flux_correction(),
        }
    }
}
