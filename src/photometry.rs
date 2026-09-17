use std::f64::consts::PI;

use crate::{Eulumdat, Symmetry};

impl Eulumdat {
    #[must_use]
    /// Calculates total luminous output from the stored intensity distribution.
    pub fn total_output(&self) -> f64 {
        self.weighted_plane_sum(Self::output_for_plane)
    }

    #[must_use]
    /// Calculates the downward flux fraction from the stored intensity distribution.
    pub fn calculated_downward_flux_fraction(&self) -> f64 {
        let output = self.total_output();
        if output <= 0.0 {
            return 0.0;
        }
        100.0 * self.weighted_plane_sum(Self::downward_for_plane) / output
    }

    #[must_use]
    /// Calculates the 50% beam angle across the C0/C180 plane pair.
    pub fn beam_angle_c0_c180(&self) -> Option<f64> {
        self.angle_for_plane(0.0, 180.0, 0.5)
    }

    #[must_use]
    /// Calculates the 50% beam angle across the C90/C270 plane pair.
    pub fn beam_angle_c90_c270(&self) -> Option<f64> {
        self.angle_for_plane(90.0, 270.0, 0.5)
    }

    #[must_use]
    /// Calculates the 10% field angle across the C0/C180 plane pair.
    pub fn field_angle_c0_c180(&self) -> Option<f64> {
        self.angle_for_plane(0.0, 180.0, 0.1)
    }

    #[must_use]
    /// Calculates the 10% field angle across the C90/C270 plane pair.
    pub fn field_angle_c90_c270(&self) -> Option<f64> {
        self.angle_for_plane(90.0, 270.0, 0.1)
    }

    fn weighted_plane_sum(&self, plane_fn: impl Fn(&Self, usize) -> f64) -> f64 {
        let stored = self.intensities.len();
        if stored == 0 || self.gamma_angles.len() < 2 || self.c_planes.is_empty() {
            return 0.0;
        }

        let mut d = 0.0;
        match self.symmetry {
            Symmetry::None => {
                for i in 1..stored {
                    d += (self.c_planes[i] - self.c_planes[i - 1]) * plane_fn(self, i - 1);
                }
                d += (360.0 - self.c_planes[stored - 1]) * plane_fn(self, stored - 1);
                d / 360.0
            }
            Symmetry::Rotational => plane_fn(self, 0),
            Symmetry::C0C180 => {
                for i in 1..stored {
                    d += 2.0 * (self.c_planes[i] - self.c_planes[i - 1]) * plane_fn(self, i - 1);
                }
                d += 2.0 * (180.0 - self.c_planes[stored - 1]) * plane_fn(self, stored - 1);
                d / 360.0
            }
            Symmetry::C90C270 => {
                // Rows run from C270 over C0 to C90; offsets are measured from C270.
                let offsets = self.c90_c270_row_offsets();
                if offsets.len() < stored {
                    return 0.0;
                }
                for i in 1..stored {
                    d += 2.0 * (offsets[i] - offsets[i - 1]) * plane_fn(self, i - 1);
                }
                d += 2.0 * (180.0 - offsets[stored - 1]) * plane_fn(self, stored - 1);
                d / 360.0
            }
            Symmetry::C0C180AndC90C270 => {
                for i in 1..stored {
                    d += 4.0 * (self.c_planes[i] - self.c_planes[i - 1]) * plane_fn(self, i - 1);
                }
                d += 4.0 * (90.0 - self.c_planes[stored - 1]) * plane_fn(self, stored - 1);
                d / 360.0
            }
        }
    }

    fn output_for_plane(&self, c: usize) -> f64 {
        let mut sum = 0.0;
        for i in 1..self.gamma_angles.len() {
            let omega = 2.0
                * PI
                * (self.gamma_angles[i - 1].to_radians().cos()
                    - self.gamma_angles[i].to_radians().cos());
            let avg = (self.intensities[c][i - 1] + self.intensities[c][i]) / 2.0;
            sum += omega * avg;
        }
        sum / 10.0
    }

    fn downward_for_plane(&self, c: usize) -> f64 {
        let mut sum = 0.0;
        for i in 1..self.gamma_angles.len() {
            if self.gamma_angles[i] > 90.0 {
                break;
            }
            let omega = 2.0
                * PI
                * (self.gamma_angles[i - 1].to_radians().cos()
                    - self.gamma_angles[i].to_radians().cos());
            let avg = (self.intensities[c][i - 1] + self.intensities[c][i]) / 2.0;
            sum += omega * avg;
        }
        sum / 10.0
    }

    /// Returns the stored intensity profile for an extended C angle.
    ///
    /// See [`Eulumdat::stored_row_for_c_plane`] for the mapping from C angle to
    /// stored row.
    pub(crate) fn intensity_profile_for_c_plane(&self, c_plane: f64) -> Option<&[f64]> {
        if self.gamma_angles.is_empty() {
            return None;
        }
        self.intensities
            .get(self.stored_row_for_c_plane(c_plane)?)
            .filter(|row| row.len() == self.gamma_angles.len())
            .map(Vec::as_slice)
    }

    fn angle_for_plane(&self, c_plane_a: f64, c_plane_b: f64, peak_fraction: f64) -> Option<f64> {
        if self.gamma_angles.len() < 2 || !(0.0..1.0).contains(&peak_fraction) {
            return None;
        }
        let profile_a = self.intensity_profile_for_c_plane(c_plane_a)?;
        let profile_b = self.intensity_profile_for_c_plane(c_plane_b)?;

        let mut theta = Vec::with_capacity(2 * self.gamma_angles.len() - 1);
        let mut intensity = Vec::with_capacity(2 * self.gamma_angles.len() - 1);
        for i in (1..self.gamma_angles.len()).rev() {
            theta.push(-self.gamma_angles[i]);
            intensity.push(profile_b[i]);
        }
        theta.push(0.0);
        intensity.push((profile_a[0] + profile_b[0]) / 2.0);
        for (i, value) in profile_a.iter().enumerate().skip(1) {
            theta.push(self.gamma_angles[i]);
            intensity.push(*value);
        }

        let full_circle = (self.gamma_angles[self.gamma_angles.len() - 1] - 180.0).abs() < 1e-6
            && self.gamma_angles[0].abs() < 1e-6;
        let (peak_idx, peak) = intensity
            .iter()
            .copied()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(&b.1))?;
        if peak <= 0.0 {
            return None;
        }
        let threshold = peak * peak_fraction;
        let right = walk_outward(&theta, &intensity, peak_idx, threshold, full_circle, 1)?;
        let left = walk_outward(&theta, &intensity, peak_idx, threshold, full_circle, -1)?;
        Some(right + left)
    }
}

fn walk_outward(
    theta: &[f64],
    intensity: &[f64],
    peak_idx: usize,
    threshold: f64,
    full_circle: bool,
    direction: i32,
) -> Option<f64> {
    let mut idx = peak_idx;
    let mut prev_i = intensity[idx];
    let mut cumulative = 0.0;
    loop {
        let mut next = idx as i32 + direction;
        if next < 0 {
            if !full_circle {
                return None;
            }
            next = theta.len() as i32 - 2;
        } else if next >= theta.len() as i32 {
            if !full_circle {
                return None;
            }
            next = 1;
        }
        let next = usize::try_from(next).ok()?;
        let mut step = if direction > 0 {
            theta[next] - theta[idx]
        } else {
            theta[idx] - theta[next]
        };
        if step <= 0.0 {
            step += 360.0;
        }
        let next_i = intensity[next];
        if prev_i >= threshold && next_i < threshold {
            let t = (prev_i - threshold) / (prev_i - next_i);
            return Some(cumulative + t * step);
        }
        cumulative += step;
        if cumulative > 180.0 + 1e-9 {
            return None;
        }
        prev_i = next_i;
        idx = next;
        if idx == peak_idx {
            return None;
        }
    }
}
