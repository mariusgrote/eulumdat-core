use std::f64::consts::PI;

use crate::Eulumdat;

impl Eulumdat {
    /// Interpolates the stored intensity in cd/klm at an arbitrary direction.
    ///
    /// Uses bilinear interpolation over the expanded C-plane grid and the gamma
    /// angles. C wraps around modulo 360; gamma outside the stored range yields
    /// `None`.
    pub(crate) fn intensity_at(&self, c_deg: f64, gamma_deg: f64) -> Option<f64> {
        self.interpolate_on_grid(c_deg, gamma_deg, |_, profile, gamma_idx| profile[gamma_idx])
    }

    /// Calculates the luminous area in m² seen from direction (C, gamma).
    ///
    /// `A = A_bottom * cos(gamma) + A_side(C) * sin(gamma)`, as in
    /// eulumdat-luminance. A luminous area width of 0 means a circular area with
    /// the length as diameter, whose side uses only the C0 height. Otherwise the
    /// side area combines `length * |cos C|` and `width * |sin C|` with the
    /// heights of the quadrant containing C.
    pub(crate) fn projected_area(&self, c_deg: f64, gamma_deg: f64) -> f64 {
        let length = self.luminous_area_length / 1000.0;
        let width = self.luminous_area_width / 1000.0;
        let height_c0 = self.luminous_area_height_c0 / 1000.0;
        let gamma = gamma_deg.to_radians();

        if self.luminous_area_width == 0.0 {
            let bottom = PI * (length / 2.0).powi(2);
            return bottom * gamma.cos() + length * height_c0 * gamma.sin();
        }

        let height_c90 = self.luminous_area_height_c90 / 1000.0;
        let height_c180 = self.luminous_area_height_c180 / 1000.0;
        let height_c270 = self.luminous_area_height_c270 / 1000.0;
        let c = c_deg.rem_euclid(360.0);
        let height_length = if (90.0..270.0).contains(&c) {
            height_c180
        } else {
            height_c0
        };
        let height_width = if c < 180.0 { height_c90 } else { height_c270 };
        let side = length * c.to_radians().cos().abs() * height_length
            + width * c.to_radians().sin().abs() * height_width;
        length * width * gamma.cos() + side * gamma.sin()
    }

    /// Interpolates the luminance in cd/m² at an arbitrary direction.
    ///
    /// Like eulumdat-luminance, luminance is first evaluated on the file's
    /// (C, gamma) grid as `I / A_proj` and then interpolated bilinearly. Grid
    /// points with a non-positive projected area count as 0 cd/m². Intensities
    /// are scaled with the flux of the first lamp set only.
    ///
    /// Returns `None` if the luminous area is zero, no lamp set exists, or
    /// gamma lies outside the stored range.
    pub(crate) fn luminance_at(&self, c_deg: f64, gamma_deg: f64) -> Option<f64> {
        // At gamma 0 the projected area is the bottom area.
        if self.projected_area(0.0, 0.0) <= 0.0 {
            return None;
        }
        let flux_klm = self.lamps.first()?.total_luminous_flux / 1000.0;

        self.interpolate_on_grid(c_deg, gamma_deg, |c_plane, profile, gamma_idx| {
            let area = self.projected_area(c_plane, self.gamma_angles[gamma_idx]);
            if area > 0.0 {
                profile[gamma_idx] * flux_klm / area
            } else {
                0.0
            }
        })
    }

    /// Bilinearly interpolates `value` between the four surrounding grid points.
    ///
    /// `value` receives the C-plane angle of the grid point, the intensity
    /// profile of that plane, and the gamma index.
    fn interpolate_on_grid(
        &self,
        c_deg: f64,
        gamma_deg: f64,
        value: impl Fn(f64, &[f64], usize) -> f64,
    ) -> Option<f64> {
        if !c_deg.is_finite() || self.c_planes.is_empty() {
            return None;
        }
        let gamma = Bracket::linear(&self.gamma_angles, gamma_deg)?;
        let c = Bracket::periodic(&self.c_planes, c_deg.rem_euclid(360.0));

        let plane_value = |c_idx: usize| -> Option<f64> {
            let c_plane = self.c_planes[c_idx];
            let profile = self.intensity_profile_for_c_plane(c_plane)?;
            let lower = value(c_plane, profile, gamma.lower);
            let upper = value(c_plane, profile, gamma.upper);
            Some(gamma.mix(lower, upper))
        };
        let lower = plane_value(c.lower)?;
        let upper = if c.upper == c.lower {
            lower
        } else {
            plane_value(c.upper)?
        };
        Some(c.mix(lower, upper))
    }
}

/// Indices of the two grid points around a value and the weight of the upper one.
#[derive(Debug, Clone, Copy)]
struct Bracket {
    lower: usize,
    upper: usize,
    weight: f64,
}

impl Bracket {
    /// Brackets `value` on an ascending axis; `None` outside the axis range.
    fn linear(axis: &[f64], value: f64) -> Option<Self> {
        let (&first, &last) = (axis.first()?, axis.last()?);
        if !(first..=last).contains(&value) {
            return None;
        }
        let lower = axis.partition_point(|angle| *angle <= value) - 1;
        if lower == axis.len() - 1 {
            return Some(Self::exact(lower));
        }
        let upper = lower + 1;
        Some(Self {
            lower,
            upper,
            weight: (value - axis[lower]) / (axis[upper] - axis[lower]),
        })
    }

    /// Brackets `value` in [0, 360) on an ascending C-plane axis that wraps at 360.
    fn periodic(axis: &[f64], value: f64) -> Self {
        let count = axis.len();
        let after = axis.partition_point(|angle| *angle <= value);
        let (lower, upper) = ((after + count - 1) % count, after % count);
        let lower_angle = if after == 0 {
            axis[lower] - 360.0
        } else {
            axis[lower]
        };
        let upper_angle = if after == count {
            axis[upper] + 360.0
        } else {
            axis[upper]
        };
        let span = upper_angle - lower_angle;
        if value == lower_angle || span <= 0.0 {
            return Self::exact(lower);
        }
        Self {
            lower,
            upper,
            weight: (value - lower_angle) / span,
        }
    }

    fn exact(index: usize) -> Self {
        Self {
            lower: index,
            upper: index,
            weight: 0.0,
        }
    }

    fn mix(self, lower: f64, upper: f64) -> f64 {
        lower + self.weight * (upper - lower)
    }
}
