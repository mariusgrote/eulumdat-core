use crate::{EulumdatError, Symmetry};

#[derive(Debug, Clone, PartialEq)]
/// Photometric distribution parsed from a tab-separated intensity table.
pub struct TableDistribution {
    /// Inferred symmetry of the table.
    pub symmetry: Symmetry,
    /// Inferred angular step between C-planes, in degrees.
    pub c_plane_step: f64,
    /// Inferred angular step between gamma angles, in degrees.
    pub gamma_step: f64,
    /// Expanded C-plane angles, in degrees.
    pub c_planes: Vec<f64>,
    /// Gamma angles, in degrees.
    pub gamma_angles: Vec<f64>,
    /// Stored luminous intensity rows indexed by C-plane, then gamma angle.
    pub intensities: Vec<Vec<f64>>,
}

impl TableDistribution {
    #[must_use]
    /// Returns the number of expanded C-plane angles.
    pub fn c_plane_count(&self) -> usize {
        self.c_planes.len()
    }

    #[must_use]
    /// Returns the number of gamma angles in each intensity row.
    pub fn gamma_count(&self) -> usize {
        self.gamma_angles.len()
    }
}

/// Parses a tab-separated intensity table into a photometric distribution.
pub fn parse_table_text(input: &str) -> Result<TableDistribution, EulumdatError> {
    let rows: Vec<Vec<&str>> = input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split('\t').collect())
        .collect();
    if rows.len() < 2 || rows[0].len() < 2 {
        return table_error("None useful data");
    }
    let input_c_count = rows[0].len() - 1;
    let input_g_count = rows.len() - 1;
    if input_c_count < 1 || input_g_count < 4 {
        return table_error("Lack of useful data");
    }
    if rows.iter().any(|row| row.len() != input_c_count + 1) {
        return table_error("Lack of useful data");
    }

    let mut c_values = Vec::with_capacity(input_c_count);
    for cell in rows[0].iter().skip(1) {
        c_values.push(parse_cell(cell)?);
    }

    let mut gamma_values = Vec::with_capacity(input_g_count);
    let mut intensities = vec![Vec::with_capacity(input_g_count); input_c_count];
    for row in rows.iter().skip(1) {
        gamma_values.push(parse_cell(row[0])?);
        for (c, cell) in row.iter().skip(1).enumerate() {
            intensities[c].push(parse_cell(cell)?);
        }
    }

    ensure_in_range(
        c_values
            .iter()
            .chain(gamma_values.iter())
            .chain(intensities.iter().flatten()),
    )?;
    if !c_values.windows(2).all(|pair| pair[0] < pair[1]) {
        return table_error("C-planes not sorted");
    }
    if !gamma_values.windows(2).all(|pair| pair[0] < pair[1]) {
        return table_error("Gamma-planes not sorted");
    }
    if gamma_values[0] < 0.0 || gamma_values[gamma_values.len() - 1] > 180.0 {
        return table_error("Wrong Gamma-planes scheme");
    }
    if intensities.iter().flatten().all(|value| *value < 1.0) {
        return table_error("Values are too low");
    }

    let gamma_step = regular_step(&gamma_values);
    let (symmetry, c_plane_step, expanded_c_planes) = infer_c_planes(&c_values)?;

    Ok(TableDistribution {
        symmetry,
        c_plane_step,
        gamma_step,
        c_planes: expanded_c_planes,
        gamma_angles: gamma_values,
        intensities,
    })
}

fn parse_cell(cell: &str) -> Result<f64, EulumdatError> {
    let normalized = cell.trim().replace(',', ".");
    let value = normalized
        .parse::<f64>()
        .map_err(|_| EulumdatError::Validation("Data are not real numbers".to_string()))?;
    Ok(value)
}

fn ensure_in_range<'a>(values: impl Iterator<Item = &'a f64>) -> Result<(), EulumdatError> {
    for value in values {
        if !(0.0..=1_000_000.0).contains(value) {
            return table_error("Values out of range");
        }
    }
    Ok(())
}

fn infer_c_planes(c: &[f64]) -> Result<(Symmetry, f64, Vec<f64>), EulumdatError> {
    let n = c.len() + 1;
    if c[0] == 0.0 && n == 2 {
        return Ok((Symmetry::Rotational, 0.0, c.to_vec()));
    }
    if c[0] == 0.0 && c[c.len() - 1] > 270.0 && n > 4 {
        require(c, 90.0, "Missing C90-plane")?;
        require(c, 180.0, "Missing C180-plane")?;
        require(c, 270.0, "Missing C270-plane")?;
        return Ok((Symmetry::None, regular_step(c), c.to_vec()));
    }
    if c[0] == 0.0 && c[c.len() - 1] == 180.0 && n > 3 {
        require(c, 90.0, "Missing C90-plane")?;
        let mut expanded = Vec::with_capacity(2 * c.len() - 2);
        expanded.extend_from_slice(c);
        for j in (1..c.len() - 1).rev() {
            expanded.push(360.0 - c[j]);
        }
        return Ok((Symmetry::C0C180, regular_step(c), expanded));
    }
    if c[0] == 0.0 && c[c.len() - 1] == 90.0 && n > 2 {
        let mut expanded = Vec::with_capacity(4 * c.len() - 4);
        expanded.extend_from_slice(c);
        for j in (0..c.len() - 1).rev() {
            expanded.push(180.0 - c[j]);
        }
        for j in (1..expanded.len() - 1).rev() {
            expanded.push(360.0 - expanded[j]);
        }
        return Ok((Symmetry::C0C180AndC90C270, regular_step(c), expanded));
    }
    if c[0] == 90.0 && c[c.len() - 1] == 270.0 && n > 3 {
        require(c, 180.0, "Missing C180-plane")?;
        let mut expanded = Vec::with_capacity(2 * c.len() - 2);
        let mut j = c.len() - 1;
        while c[j] != 180.0 {
            j -= 1;
        }
        for k in (1..=j).rev() {
            expanded.push(180.0 - c[k]);
        }
        expanded.extend_from_slice(c);
        for k in ((j + 1)..(c.len() - 1)).rev() {
            expanded.push(540.0 - c[k]);
        }
        return Ok((Symmetry::C90C270, regular_step(c), expanded));
    }
    table_error("Wrong C-planes scheme")
}

fn require(values: &[f64], expected: f64, message: &str) -> Result<(), EulumdatError> {
    if values.contains(&expected) {
        Ok(())
    } else {
        table_error(message)
    }
}

fn regular_step(values: &[f64]) -> f64 {
    if values.len() <= 1 {
        return 0.0;
    }
    let step = values[1] - values[0];
    if values.windows(2).all(|pair| (pair[1] - pair[0]) == step) {
        step
    } else {
        0.0
    }
}

fn table_error<T>(message: &str) -> Result<T, EulumdatError> {
    Err(EulumdatError::Validation(message.to_string()))
}
