use super::tables::{GUTH_POSITION_INDEX, GUTH_STEPS_PER_UNIT};

/// Interpolates the Guth position index p at (|H/R|, |T/R|).
///
/// Port of eulumdat-ugr `GuthTable.p`, which uses scipy's
/// `RegularGridInterpolator(method="linear", bounds_error=False,
/// fill_value=nan)`. The result is NaN outside the table and whenever one of
/// the four surrounding cells is NaN, even if its weight is zero. A luminaire
/// with a NaN index is left out of the UGR sum.
pub(crate) fn position_index(h_r: f64, t_r: f64) -> f64 {
    let rows = GUTH_POSITION_INDEX.len();
    let columns = GUTH_POSITION_INDEX[0].len();
    let (Some(row), Some(column)) = (Cell::find(t_r.abs(), rows), Cell::find(h_r.abs(), columns))
    else {
        return f64::NAN;
    };

    let upper_row = &GUTH_POSITION_INDEX[row.index + 1];
    let lower_row = &GUTH_POSITION_INDEX[row.index];
    let lower = column.mix(lower_row[column.index], lower_row[column.index + 1]);
    let upper = column.mix(upper_row[column.index], upper_row[column.index + 1]);
    // NaN corners propagate through the arithmetic, also with zero weight.
    row.mix(lower, upper)
}

/// Lower grid index of the interpolation cell and the weight of the upper point.
#[derive(Debug, Clone, Copy)]
struct Cell {
    index: usize,
    weight: f64,
}

impl Cell {
    /// Finds the cell like scipy: the largest `i` with `axis[i] <= value`,
    /// capped at `len - 2`. `None` outside the axis or for NaN.
    fn find(value: f64, len: usize) -> Option<Self> {
        let last = axis_value(len - 1);
        if !(0.0..=last).contains(&value) {
            return None;
        }
        // Start from the arithmetic guess and correct for rounding, so the
        // comparison uses the exact axis values.
        let mut index = ((value * GUTH_STEPS_PER_UNIT).floor() as usize).min(len - 1);
        while index > 0 && axis_value(index) > value {
            index -= 1;
        }
        while index + 1 < len && axis_value(index + 1) <= value {
            index += 1;
        }
        let index = index.min(len - 2);
        let (lower, upper) = (axis_value(index), axis_value(index + 1));
        Some(Self {
            index,
            weight: (value - lower) / (upper - lower),
        })
    }

    fn mix(self, lower: f64, upper: f64) -> f64 {
        lower * (1.0 - self.weight) + upper * self.weight
    }
}

/// Axis value at `index`, as numpy's `round(arange(0, n, 0.1), 2)` produces it.
fn axis_value(index: usize) -> f64 {
    index as f64 / GUTH_STEPS_PER_UNIT
}
