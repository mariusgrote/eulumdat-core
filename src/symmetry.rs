//! Mapping between C-plane angles and stored intensity rows.
//!
//! Three angles are involved when looking up the intensity row for a
//! direction:
//!
//! 1. the *extended* C angle, any finite value in degrees, wrapping at 360°;
//! 2. the *representative* angle, the extended angle mirrored onto the part of
//!    the circle whose rows are stored for the symmetry
//!    ([`Symmetry::representative_c_angle`]);
//! 3. the *stored row*, the index into [`Eulumdat::intensities`]
//!    ([`Eulumdat::stored_row_for_c_plane`]).
//!
//! | Symmetry | Stored part | Mirror | Row order |
//! |---|---|---|---|
//! | `None` (ISYM 0) | full circle | – | position in `c_planes` |
//! | `Rotational` (ISYM 1) | C0 | every C | single row |
//! | `C0C180` (ISYM 2) | C0…C180 | C → 360° − C | position in `c_planes` |
//! | `C90C270` (ISYM 3) | C270…C360/C0…C90 | C → 180° − C | C270, …, C0, …, C90 |
//! | `C0C180AndC90C270` (ISYM 4) | C0…C90 | both mirrors | position in `c_planes` |
//!
//! ISYM 3 is the only symmetry whose rows do not start at C0: EULUMDAT stores
//! them from C270 across the 0°/360° wrap to C90, while the C-plane angles in
//! the file still start at C0. The row of a plane is therefore its rank by
//! the offset `(C − 270°) mod 360°`, which runs from 0° (C270) over 90° (C0)
//! to 180° (C90). This does not assume equidistant planes.

use crate::{Eulumdat, Symmetry};

/// Tolerance for matching C angles, in degrees.
const C_ANGLE_TOLERANCE_DEG: f64 = 1e-6;
/// First stored C angle for [`Symmetry::C90C270`], in degrees.
const C90_C270_FIRST_ROW_DEG: f64 = 270.0;

impl Symmetry {
    /// Mirrors an extended C angle onto the part of the circle stored for this
    /// symmetry.
    ///
    /// The result lies in [0°, 360°) for `None`, is 0° for `Rotational`, lies
    /// in [0°, 180°] for `C0C180`, in [270°, 360°) ∪ [0°, 90°] for `C90C270`,
    /// and in [0°, 90°] for `C0C180AndC90C270`.
    pub(crate) fn representative_c_angle(self, c_deg: f64) -> f64 {
        let c = c_deg.rem_euclid(360.0);
        match self {
            Self::None => c,
            Self::Rotational => 0.0,
            Self::C0C180 => mirror_to_c0_c180(c),
            Self::C90C270 => mirror_to_c270_c90(c),
            Self::C0C180AndC90C270 => {
                let half = mirror_to_c0_c180(c);
                if half > 90.0 { 180.0 - half } else { half }
            }
        }
    }
}

impl Eulumdat {
    /// Returns the index into [`Eulumdat::intensities`] that stores the
    /// profile for an extended C angle.
    ///
    /// The representative angle must match one of the C-plane angles within
    /// a small tolerance; there is no interpolation between planes. Returns
    /// `None` if no plane matches or the row is not stored.
    pub(crate) fn stored_row_for_c_plane(&self, c_deg: f64) -> Option<usize> {
        if !c_deg.is_finite() {
            return None;
        }
        let representative = self.symmetry.representative_c_angle(c_deg);
        let row = match self.symmetry {
            Symmetry::Rotational => 0,
            Symmetry::None | Symmetry::C0C180 | Symmetry::C0C180AndC90C270 => self
                .c_planes
                .iter()
                .position(|plane| same_c_angle(*plane, representative))?,
            Symmetry::C90C270 => {
                let offset = c90_c270_offset(representative);
                self.c90_c270_row_offsets()
                    .iter()
                    .position(|row_offset| (row_offset - offset).abs() < C_ANGLE_TOLERANCE_DEG)?
            }
        };
        (row < self.intensities.len()).then_some(row)
    }

    /// Offsets `(C − 270°) mod 360°` of the [`Symmetry::C90C270`] rows, in
    /// row order.
    ///
    /// Collects the C-plane angles within C270…C90 (across 0°), orders them by
    /// offset and drops duplicates such as C0 and C360. Row `i` stores the
    /// plane at `(270° + offsets[i]) mod 360°`.
    pub(crate) fn c90_c270_row_offsets(&self) -> Vec<f64> {
        let mut offsets: Vec<f64> = self
            .c_planes
            .iter()
            .filter(|plane| plane.is_finite())
            .map(|plane| c90_c270_offset(*plane))
            .filter(|offset| *offset <= 180.0 + C_ANGLE_TOLERANCE_DEG)
            .collect();
        offsets.sort_by(f64::total_cmp);
        offsets.dedup_by(|next, kept| (*next - *kept).abs() < C_ANGLE_TOLERANCE_DEG);
        offsets
    }
}

/// Mirrors C in [0°, 360°) at the C0–C180 plane onto [0°, 180°].
fn mirror_to_c0_c180(c: f64) -> f64 {
    if c > 180.0 { 360.0 - c } else { c }
}

/// Mirrors C in [0°, 360°) at the C90–C270 plane onto C270…C360/C0…C90.
fn mirror_to_c270_c90(c: f64) -> f64 {
    if c > 90.0 && c < 270.0 {
        (180.0 - c).rem_euclid(360.0)
    } else {
        c
    }
}

/// Offset of a C angle from C270 in [0°, 360°); values just below 360° count
/// as C270 itself.
fn c90_c270_offset(c: f64) -> f64 {
    let offset = (c - C90_C270_FIRST_ROW_DEG).rem_euclid(360.0);
    if offset > 360.0 - C_ANGLE_TOLERANCE_DEG {
        0.0
    } else {
        offset
    }
}

/// `true` if two C angles denote the same plane, including across 360°.
fn same_c_angle(a: f64, b: f64) -> bool {
    let diff = (a - b).rem_euclid(360.0);
    diff.min(360.0 - diff) < C_ANGLE_TOLERANCE_DEG
}

#[cfg(test)]
mod tests;
