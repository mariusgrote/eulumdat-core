use crate::{Distribution, Eulumdat, EulumdatError};

impl Eulumdat {
    /// Resamples gamma angles to a regular step using cubic spline interpolation.
    pub fn resample_gamma(&mut self, new_step_degrees: u32) -> Result<(), EulumdatError> {
        if !(1..=180).contains(&new_step_degrees) {
            return Err(EulumdatError::Validation(
                "New gamma step must be between 1 and 180 degrees.".to_string(),
            ));
        }
        if self.gamma_angles.len() < 2 {
            return Err(EulumdatError::Validation(
                "Not enough gamma points to resample.".to_string(),
            ));
        }
        if !self.gamma_angles.windows(2).all(|pair| pair[1] > pair[0]) {
            return Err(EulumdatError::Validation(
                "Gamma-planes not strictly increasing. Cannot resample.".to_string(),
            ));
        }

        let min_g = self.gamma_angles[0];
        let max_g = self.gamma_angles[self.gamma_angles.len() - 1];
        let step = f64::from(new_step_degrees);
        let mut new_gamma = vec![min_g];
        let mut g = min_g + step;
        while g < max_g {
            new_gamma.push(g);
            g += step;
        }
        if (new_gamma[new_gamma.len() - 1] - max_g).abs() > 1e-6 {
            new_gamma.push(max_g);
        }

        let mut new_intensities = Vec::with_capacity(self.intensities.len());
        for row in &self.intensities {
            let mut values = cubic_spline_evaluate(&self.gamma_angles, row, &new_gamma);
            for value in &mut values {
                if *value < 0.0 {
                    *value = 0.0;
                }
                *value = round_to_one_decimal(*value);
            }
            new_intensities.push(values);
        }

        self.replace_distribution(Distribution {
            symmetry: self.symmetry,
            c_plane_step: self.c_plane_step,
            gamma_step: step,
            c_planes: self.c_planes.clone(),
            gamma_angles: new_gamma,
            intensities: new_intensities,
        })
    }

    /// Scales intensities so calculated total output is 100%.
    pub fn scale_to_100_percent(&mut self) {
        let current_output = self.total_output();
        if current_output <= 0.0 {
            return;
        }
        let scale_factor = 100.0 / current_output;
        for row in &mut self.intensities {
            for value in row {
                *value = round_to_one_decimal(*value * scale_factor);
            }
        }
        self.light_output_ratio = 100.0;
    }

    /// Scales intensities to 100% and adjusts lamp flux to preserve total output.
    pub fn scale_to_100_percent_with_flux(&mut self) {
        let current_output = self.total_output();
        if current_output <= 0.0 {
            return;
        }
        let scale_factor = 100.0 / current_output;
        let efficiency_factor = current_output / 100.0;
        for row in &mut self.intensities {
            for value in row {
                *value = round_to_one_decimal(*value * scale_factor);
            }
        }
        for lamp in &mut self.lamps {
            lamp.total_luminous_flux = (lamp.total_luminous_flux * efficiency_factor).round();
        }
        self.light_output_ratio = 100.0;
    }
}

fn cubic_spline_evaluate(x: &[f64], y: &[f64], xq: &[f64]) -> Vec<f64> {
    let n = x.len();
    if n < 2 || xq.is_empty() {
        return Vec::new();
    }
    if n == 2 {
        return xq
            .iter()
            .map(|value| linear_between(x[0], x[1], y[0], y[1], *value))
            .collect();
    }

    let mut u = vec![0.0; n - 1];
    let mut z = vec![0.0; n];
    for i in 1..n - 1 {
        let hi = x[i] - x[i - 1];
        let hip1 = x[i + 1] - x[i];
        if hi <= 0.0 || hip1 <= 0.0 {
            return xq
                .iter()
                .map(|value| linear_fallback(x, y, *value))
                .collect();
        }
        let sig = hi / (hi + hip1);
        let p = sig * z[i - 1] + 2.0;
        z[i] = (sig - 1.0) / p;
        let d = (y[i + 1] - y[i]) / hip1 - (y[i] - y[i - 1]) / hi;
        u[i] = (6.0 * d / (hi + hip1) - sig * u[i - 1]) / p;
    }
    z[n - 1] = 0.0;
    for j in (0..=n - 2).rev() {
        z[j] = z[j] * z[j + 1] + u[j];
    }

    xq.iter()
        .map(|xval| evaluate_spline_point(x, y, &z, *xval))
        .collect()
}

fn evaluate_spline_point(x: &[f64], y: &[f64], z: &[f64], xval: f64) -> f64 {
    let n = x.len();
    if xval <= x[0] {
        return linear_between(x[0], x[1], y[0], y[1], xval);
    }
    if xval >= x[n - 1] {
        return linear_between(x[n - 2], x[n - 1], y[n - 2], y[n - 1], xval);
    }
    let mut lo = 0;
    let mut hi = n - 1;
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if x[mid] > xval {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let h = x[lo + 1] - x[lo];
    if h <= 0.0 {
        return linear_between(x[lo], x[lo + 1], y[lo], y[lo + 1], xval);
    }
    let a = (x[lo + 1] - xval) / h;
    let b = (xval - x[lo]) / h;
    a * y[lo]
        + b * y[lo + 1]
        + ((a * a * a - a) * z[lo] + (b * b * b - b) * z[lo + 1]) * (h * h) / 6.0
}

fn linear_fallback(x: &[f64], y: &[f64], xval: f64) -> f64 {
    if xval <= x[0] {
        return y[0];
    }
    if xval >= x[x.len() - 1] {
        return y[y.len() - 1];
    }
    let mut j = 1;
    while j < x.len() && x[j] < xval {
        j += 1;
    }
    linear_between(x[j - 1], x[j], y[j - 1], y[j], xval)
}

fn linear_between(x0: f64, x1: f64, y0: f64, y1: f64, xval: f64) -> f64 {
    let dx = x1 - x0;
    let slope = if dx.abs() > 0.0 { (y1 - y0) / dx } else { 0.0 };
    y0 + slope * (xval - x0)
}

fn round_to_one_decimal(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
