use super::UgrView;

/// Mounting height H of the luminaires above the observer's eye, in m.
///
/// Fixed by CIE 190:2010 §4.2; the table is independent of H because all room
/// dimensions scale with it.
pub(crate) const MOUNTING_HEIGHT: f64 = 2.0;
/// Luminaire spacing S as a multiple of H for catalogue tables.
pub(crate) const CATALOGUE_SPACING_TO_HEIGHT: f64 = 0.25;
/// Luminaires above this elevation angle are ignored (CIE 117:1995 §4.5).
const GAMMA_MAX_DEG: f64 = 85.0;
/// Luminaires with a larger T/R ratio are ignored (CIE 117:1995 §4.5).
const T_R_MAX: f64 = 3.0;

/// Regular luminaire grid of one CIE 190 room, ported from eulumdat-ugr
/// `UgrGrid`.
///
/// The observer sits at the middle of one wall and looks in +Y. Positions
/// across the line of sight are T = ±S/2, ±3S/2, …; positions along it are
/// R = S/2, 3S/2, ….
#[derive(Debug, Clone, Copy)]
pub(crate) struct RoomGrid {
    /// Room width X across the line of sight, in m.
    width: f64,
    /// Room depth Y along the line of sight, in m.
    depth: f64,
    /// Luminaire spacing S, in m.
    spacing: f64,
    /// Number of T positions on each side of the line of sight.
    half_columns: usize,
    /// Number of R positions.
    rows: usize,
}

/// A luminaire position that passed the T/R and γ filters.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Luminaire {
    /// Distance along the line of sight R, in m.
    pub(crate) r: f64,
    /// Absolute distance across the line of sight |T|, in m.
    pub(crate) t: f64,
    /// Photometric C angle towards the observer, in degrees.
    pub(crate) c_deg: f64,
    /// Elevation angle γ from nadir, in degrees.
    pub(crate) gamma_deg: f64,
    /// Squared distance r² = R² + T² + H² to the observer's eye, in m².
    pub(crate) distance_squared: f64,
}

impl RoomGrid {
    /// Builds the grid for a room of `x_h` × `y_h` times the mounting height
    /// with spacing S = `spacing_to_height` · H.
    pub(crate) fn new(x_h: u8, y_h: u8, spacing_to_height: f64) -> Self {
        let width = f64::from(x_h) * MOUNTING_HEIGHT;
        let depth = f64::from(y_h) * MOUNTING_HEIGHT;
        let spacing = spacing_to_height * MOUNTING_HEIGHT;
        Self {
            width,
            depth,
            spacing,
            half_columns: (width / spacing / 2.0).round() as usize,
            rows: (depth / spacing).round() as usize,
        }
    }

    /// Total number of luminaires N in the room, before filtering.
    pub(crate) fn luminaire_count(&self) -> usize {
        2 * self.half_columns * self.rows
    }

    /// Wall area `A_w = 2·H·(X + Y)` between eye level and luminaire plane, in m².
    pub(crate) fn wall_area(&self) -> f64 {
        2.0 * MOUNTING_HEIGHT * (self.width + self.depth)
    }

    /// Luminaires on one side of the line of sight that pass the filters
    /// T/R ≤ 3 and γ ≤ 85°.
    ///
    /// Each yielded position stands for the mirrored pair at ±T, which has the
    /// same geometry; callers count it twice.
    pub(crate) fn half_luminaires(&self, view: UgrView) -> impl Iterator<Item = Luminaire> {
        let spacing = self.spacing;
        let half_columns = self.half_columns;
        (0..self.rows).flat_map(move |row| {
            let r = (row as f64 + 0.5) * spacing;
            (0..half_columns).filter_map(move |column| {
                let t = (column as f64 + 0.5) * spacing;
                let horizontal = (r * r + t * t).sqrt();
                let gamma_deg = horizontal.atan2(MOUNTING_HEIGHT).to_degrees();
                if t / r > T_R_MAX || gamma_deg > GAMMA_MAX_DEG {
                    return None;
                }
                let azimuth = t.atan2(r).to_degrees();
                let c_deg = match view {
                    // Crosswise: C = atan2(T, R); endwise: C = 90° − atan2(T, R).
                    UgrView::Crosswise => azimuth,
                    UgrView::Endwise => 90.0 - azimuth,
                };
                Some(Luminaire {
                    r,
                    t,
                    c_deg,
                    gamma_deg,
                    distance_squared: r * r + t * t + MOUNTING_HEIGHT * MOUNTING_HEIGHT,
                })
            })
        })
    }
}
