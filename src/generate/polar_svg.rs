use svg::Document;
use svg::node::element::path::Data;
use svg::node::element::{Circle, Group, Line, Path as SvgPath, Rectangle, Text};

use crate::{Eulumdat, EulumdatError};

/// Options used when rendering a polar photometric diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct PolarDiagramOptions {
    /// SVG viewport width in CSS pixels.
    pub width: u32,
    /// SVG viewport height in CSS pixels.
    pub height: u32,
    /// Padding around the circular graph area.
    pub margin: f64,
    /// Optional title rendered above the graph.
    pub title: Option<String>,
    /// C-plane pairs to render as signed gamma curves.
    pub planes: Vec<PlanePair>,
    /// Whether to draw circular grid rings.
    pub show_grid: bool,
    /// Whether to draw the curve legend.
    pub show_legend: bool,
    /// Whether to draw axis labels.
    pub show_axis_labels: bool,
    /// How luminous intensity values are scaled before plotting.
    pub intensity_mode: IntensityMode,
}

/// A pair of C-planes rendered as one signed polar curve.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanePair {
    /// C0 on the positive side and C180 on the negative side.
    C0C180,
    /// C90 on the positive side and C270 on the negative side.
    C90C270,
    /// C45 on the positive side and C225 on the negative side.
    C45C225,
    /// C135 on the positive side and C315 on the negative side.
    C135C315,
    /// Custom C-plane pair with caller-provided legend label.
    Custom {
        /// Positive-side C-plane angle in degrees.
        a: f64,
        /// Negative-side C-plane angle in degrees.
        b: f64,
        /// Legend label.
        label: String,
    },
}

/// Intensity scaling mode for generated diagrams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntensityMode {
    /// Plot values as stored in EULUMDAT, in candela per kilolumen.
    StoredCandelaPerKilolumen,
    /// Plot values multiplied by the model conversion factor.
    ConvertedByFactor,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderedPolar {
    pub(crate) document: Document,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct Curve {
    label: String,
    values: Vec<(f64, f64)>,
}

impl Default for PolarDiagramOptions {
    fn default() -> Self {
        Self {
            width: 800,
            height: 800,
            margin: 56.0,
            title: None,
            planes: vec![PlanePair::C0C180, PlanePair::C90C270],
            show_grid: true,
            show_legend: true,
            show_axis_labels: true,
            intensity_mode: IntensityMode::StoredCandelaPerKilolumen,
        }
    }
}

impl Eulumdat {
    /// Renders the model's selected polar luminous intensity curves as SVG.
    pub fn to_polar_svg(&self, options: &PolarDiagramOptions) -> Result<String, EulumdatError> {
        Ok(self.render_polar_svg(options)?.document.to_string())
    }

    pub(crate) fn render_polar_svg(
        &self,
        options: &PolarDiagramOptions,
    ) -> Result<RenderedPolar, EulumdatError> {
        let (curves, notes) = collect_curves(self, options)?;
        if curves.is_empty() {
            return Err(EulumdatError::Generation(
                "no requested C-plane pairs can be rendered".to_string(),
            ));
        }

        let max_intensity = curves
            .iter()
            .flat_map(|curve| curve.values.iter().map(|(_, value)| *value))
            .filter(|value| value.is_finite())
            .fold(0.0_f64, f64::max);
        if max_intensity <= 0.0 {
            return Err(EulumdatError::Generation(
                "selected C-plane pairs contain no positive intensities".to_string(),
            ));
        }

        let width = f64::from(options.width);
        let height = f64::from(options.height);
        let center_x = width / 2.0;
        let center_y = height / 2.0 + if options.title.is_some() { 20.0 } else { 0.0 };
        let radius = ((width.min(height) / 2.0) - options.margin).max(1.0);
        let scale_max = nice_ceiling(max_intensity);
        let colors = ["#1f77b4", "#d62728", "#2ca02c", "#9467bd", "#ff7f0e"];

        let mut doc = Document::new()
            .set("xmlns", "http://www.w3.org/2000/svg")
            .set("viewBox", (0, 0, options.width, options.height))
            .set("width", options.width)
            .set("height", options.height)
            .set("role", "img");
        doc = doc.add(
            Rectangle::new()
                .set("x", 0)
                .set("y", 0)
                .set("width", options.width)
                .set("height", options.height)
                .set("fill", "#ffffff"),
        );

        if let Some(title) = &options.title {
            doc = doc.add(
                Text::new(title.clone())
                    .set("x", center_x)
                    .set("y", 34)
                    .set("font-family", "Arial, Helvetica, sans-serif")
                    .set("font-size", 22)
                    .set("font-weight", "700")
                    .set("text-anchor", "middle")
                    .set("fill", "#111111"),
            );
        }

        let mut grid = Group::new()
            .set("id", "polar-grid")
            .set("stroke", "#d8d8d8")
            .set("stroke-width", 1)
            .set("fill", "none");
        if options.show_grid {
            for i in 1..=4 {
                let r = radius * f64::from(i) / 4.0;
                grid = grid.add(
                    Circle::new()
                        .set("cx", center_x)
                        .set("cy", center_y)
                        .set("r", r),
                );
            }
        }
        for angle in [0.0_f64, 45.0, 90.0, 135.0, 180.0, -45.0, -90.0, -135.0] {
            let (x, y) = polar_point(center_x, center_y, radius, angle);
            grid = grid.add(
                Line::new()
                    .set("x1", center_x)
                    .set("y1", center_y)
                    .set("x2", x)
                    .set("y2", y),
            );
        }
        doc = doc.add(grid);

        if options.show_axis_labels {
            doc = add_axis_labels(doc, center_x, center_y, radius);
            for i in 1..=4 {
                let value = scale_max * f64::from(i) / 4.0;
                doc = doc.add(
                    Text::new(format_number(value))
                        .set("x", center_x + 5.0)
                        .set("y", center_y - radius * f64::from(i) / 4.0 - 4.0)
                        .set("font-family", "Arial, Helvetica, sans-serif")
                        .set("font-size", 12)
                        .set("fill", "#666666"),
                );
            }
        }

        let mut curve_group = Group::new().set("id", "polar-curves").set("fill", "none");
        for (index, curve) in curves.iter().enumerate() {
            let color = colors[index % colors.len()];
            let mut data = Data::new();
            for (point_index, (theta, value)) in curve.values.iter().enumerate() {
                let r = radius * (value / scale_max).clamp(0.0, 1.0);
                let (x, y) = polar_point(center_x, center_y, r, *theta);
                data = if point_index == 0 {
                    data.move_to((x, y))
                } else {
                    data.line_to((x, y))
                };
            }
            curve_group = curve_group.add(
                SvgPath::new()
                    .set("d", data)
                    .set("stroke", color)
                    .set("stroke-width", 2.5)
                    .set("stroke-linejoin", "round")
                    .set("stroke-linecap", "round")
                    .set("data-plane-pair", curve.label.as_str()),
            );
        }
        doc = doc.add(curve_group);

        if options.show_legend {
            doc = add_legend(
                doc,
                &curves,
                &notes,
                colors,
                width - options.margin - 170.0,
                68.0,
            );
        }

        Ok(RenderedPolar {
            document: doc,
            notes,
        })
    }
}

fn collect_curves(
    model: &Eulumdat,
    options: &PolarDiagramOptions,
) -> Result<(Vec<Curve>, Vec<String>), EulumdatError> {
    let mut curves = Vec::new();
    let mut notes = Vec::new();
    let factor = match options.intensity_mode {
        IntensityMode::StoredCandelaPerKilolumen => 1.0,
        IntensityMode::ConvertedByFactor => model.conversion_factor,
    };

    for pair in &options.planes {
        let (a, b, label) = pair.parts();
        let Some(profile_a) = model.intensity_profile_for_c_plane(a) else {
            notes.push(format!("{label}: C{} unavailable", format_number(a)));
            continue;
        };
        let Some(profile_b) = model.intensity_profile_for_c_plane(b) else {
            notes.push(format!("{label}: C{} unavailable", format_number(b)));
            continue;
        };
        if profile_a.len() != model.gamma_angles.len()
            || profile_b.len() != model.gamma_angles.len()
        {
            return Err(EulumdatError::Generation(format!(
                "{label}: intensity row length does not match gamma angles"
            )));
        }

        let mut values = Vec::with_capacity(2 * model.gamma_angles.len() - 1);
        for index in (1..model.gamma_angles.len()).rev() {
            values.push((-model.gamma_angles[index], profile_b[index] * factor));
        }
        values.push((0.0, (profile_a[0] + profile_b[0]) * factor / 2.0));
        for (index, value) in profile_a.iter().enumerate().skip(1) {
            values.push((model.gamma_angles[index], *value * factor));
        }
        curves.push(Curve { label, values });
    }

    Ok((curves, notes))
}

impl PlanePair {
    fn parts(&self) -> (f64, f64, String) {
        match self {
            Self::C0C180 => (0.0, 180.0, "C0-C180".to_string()),
            Self::C90C270 => (90.0, 270.0, "C90-C270".to_string()),
            Self::C45C225 => (45.0, 225.0, "C45-C225".to_string()),
            Self::C135C315 => (135.0, 315.0, "C135-C315".to_string()),
            Self::Custom { a, b, label } => (*a, *b, label.clone()),
        }
    }
}

fn add_axis_labels(mut doc: Document, center_x: f64, center_y: f64, radius: f64) -> Document {
    for (label, angle, dx, dy) in [
        ("0°", 0.0, 0.0, 20.0),
        ("90°", 90.0, 20.0, 4.0),
        ("180°", 180.0, 0.0, -12.0),
        ("-90°", -90.0, -22.0, 4.0),
    ] {
        let (x, y) = polar_point(center_x, center_y, radius + 18.0, angle);
        doc = doc.add(
            Text::new(label)
                .set("x", x + dx)
                .set("y", y + dy)
                .set("font-family", "Arial, Helvetica, sans-serif")
                .set("font-size", 13)
                .set("text-anchor", "middle")
                .set("fill", "#555555"),
        );
    }
    doc
}

fn add_legend(
    mut doc: Document,
    curves: &[Curve],
    notes: &[String],
    colors: [&str; 5],
    x: f64,
    y: f64,
) -> Document {
    let mut legend = Group::new()
        .set("id", "polar-legend")
        .set("font-family", "Arial, Helvetica, sans-serif")
        .set("font-size", 13);
    for (index, curve) in curves.iter().enumerate() {
        let row_y = y + index as f64 * 20.0;
        legend = legend
            .add(
                Line::new()
                    .set("x1", x)
                    .set("y1", row_y - 4.0)
                    .set("x2", x + 28.0)
                    .set("y2", row_y - 4.0)
                    .set("stroke", colors[index % colors.len()])
                    .set("stroke-width", 3),
            )
            .add(
                Text::new(curve.label.clone())
                    .set("x", x + 36.0)
                    .set("y", row_y)
                    .set("fill", "#222222"),
            );
    }
    for (index, note) in notes.iter().enumerate() {
        let row_y = y + (curves.len() + index) as f64 * 20.0;
        legend = legend.add(
            Text::new(note.clone())
                .set("x", x)
                .set("y", row_y)
                .set("fill", "#777777"),
        );
    }
    doc = doc.add(legend);
    doc
}

fn polar_point(center_x: f64, center_y: f64, radius: f64, theta: f64) -> (f64, f64) {
    let radians = theta.to_radians();
    (
        center_x + radius * radians.sin(),
        center_y + radius * radians.cos(),
    )
}

fn nice_ceiling(value: f64) -> f64 {
    if value <= 0.0 || !value.is_finite() {
        return 1.0;
    }
    let exponent = value.log10().floor();
    let base = 10.0_f64.powf(exponent);
    let normalized = value / base;
    let nice = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * base
}

pub(crate) fn format_number(value: f64) -> String {
    if value.is_finite() && value.fract().abs() < 1e-9 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}
