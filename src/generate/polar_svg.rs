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
    /// Visual treatment and framing used for the diagram.
    pub presentation: PolarDiagramPresentation,
}

/// Visual treatment used for a polar diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolarDiagramPresentation {
    /// The original full-circle diagram, retained for reports and existing callers.
    Classic,
    /// A denser technical diagram that focuses downlights while preserving uplight.
    Focused,
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
    /// Plot operating cd/klm values: stored values multiplied by the model's
    /// EULUMDAT conversion factor.
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
            presentation: PolarDiagramPresentation::Classic,
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
        let focused = options.presentation == PolarDiagramPresentation::Focused;
        let downlight_framing = focused && !has_meaningful_uplight(&curves, max_intensity);
        let center_x = width / 2.0;
        let classic_center_y = height / 2.0 + if options.title.is_some() { 20.0 } else { 0.0 };
        let center_y = if downlight_framing {
            (height * 0.2)
                .max(options.margin + 12.0)
                .min(height - options.margin - 1.0)
        } else {
            classic_center_y
        };
        let radius = if downlight_framing {
            (height - center_y - options.margin).max(1.0)
        } else {
            ((width.min(height) / 2.0) - options.margin).max(1.0)
        };
        let (scale_max, tick_step) = if focused {
            focused_scale(max_intensity)
        } else {
            (nice_ceiling(max_intensity), 0.0)
        };
        let colors = ["#1f77b4", "#d62728", "#2ca02c", "#9467bd", "#ff7f0e"];

        let mut doc = Document::new()
            .set("xmlns", "http://www.w3.org/2000/svg")
            .set("viewBox", (0, 0, options.width, options.height))
            .set("width", options.width)
            .set("height", options.height)
            .set("role", "img")
            .set(
                "data-polar-presentation",
                if focused { "focused" } else { "classic" },
            )
            .set(
                "data-polar-framing",
                if downlight_framing {
                    "downlight"
                } else {
                    "full"
                },
            );
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
        if focused {
            if options.show_grid {
                let mut value = tick_step;
                while value <= scale_max + tick_step * 1e-9 {
                    grid = grid.add(
                        Circle::new()
                            .set("cx", center_x)
                            .set("cy", center_y)
                            .set("r", radius * value / scale_max),
                    );
                    value += tick_step;
                }
            }
            let inset = options.margin.min(width.min(height) / 4.0) * 0.35;
            for angle in (-180..180).step_by(15) {
                let (x, y) = ray_to_bounds(
                    center_x,
                    center_y,
                    f64::from(angle),
                    inset,
                    inset,
                    width - inset,
                    height - inset,
                );
                grid = grid.add(
                    Line::new()
                        .set("x1", center_x)
                        .set("y1", center_y)
                        .set("x2", x)
                        .set("y2", y),
                );
            }
        } else {
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
        }
        doc = doc.add(grid);

        if options.show_axis_labels {
            if focused {
                doc = add_focused_axis_labels(
                    doc,
                    center_x,
                    center_y,
                    radius,
                    width,
                    height,
                    options.margin,
                    scale_max,
                    tick_step,
                    downlight_framing,
                );
            } else {
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
        }

        let mut fill_group = Group::new().set("id", "polar-fills");
        let mut curve_group = Group::new().set("id", "polar-curves").set("fill", "none");
        for (index, curve) in curves.iter().enumerate() {
            let color = if focused {
                focused_color(curve, index)
            } else {
                colors[index % colors.len()]
            };
            if focused {
                fill_group = fill_group.add(
                    SvgPath::new()
                        .set(
                            "d",
                            curve_data(curve, center_x, center_y, radius, scale_max).close(),
                        )
                        .set("fill", "#fff28a")
                        .set("fill-opacity", 0.3)
                        .set("stroke", "none")
                        .set("data-plane-pair", curve.label.as_str()),
                );
            }
            curve_group = curve_group.add(
                SvgPath::new()
                    .set(
                        "d",
                        curve_data(curve, center_x, center_y, radius, scale_max),
                    )
                    .set("stroke", color)
                    .set("stroke-width", if focused { 2.0 } else { 2.5 })
                    .set("stroke-linejoin", "round")
                    .set("stroke-linecap", "round")
                    .set("data-plane-pair", curve.label.as_str()),
            );
        }
        if focused {
            doc = doc.add(fill_group);
        }
        doc = doc.add(curve_group);

        if options.show_legend {
            doc = add_legend(
                doc,
                &curves,
                &notes,
                colors,
                focused,
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
    focused: bool,
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
                    .set(
                        "stroke",
                        if focused {
                            focused_color(curve, index)
                        } else {
                            colors[index % colors.len()]
                        },
                    )
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

fn curve_data(curve: &Curve, center_x: f64, center_y: f64, radius: f64, scale_max: f64) -> Data {
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
    data
}

fn has_meaningful_uplight(curves: &[Curve], max_intensity: f64) -> bool {
    let threshold = max_intensity * 0.01;
    curves.iter().any(|curve| {
        curve
            .values
            .iter()
            .any(|(theta, value)| theta.abs() > 90.0 && *value > threshold)
    })
}

fn focused_color(curve: &Curve, index: usize) -> &'static str {
    if curve.label == "C0-C180" {
        "#ff6b6b"
    } else if curve.label == "C90-C270" {
        "#7375ff"
    } else if curve.label == "C45-C225" {
        "#2f9e62"
    } else if curve.label == "C135-C315" {
        "#e89032"
    } else {
        ["#8f63c7", "#2c9aa0", "#b56a9b"][index % 3]
    }
}

#[allow(clippy::too_many_arguments)]
fn add_focused_axis_labels(
    mut doc: Document,
    center_x: f64,
    center_y: f64,
    radius: f64,
    width: f64,
    height: f64,
    margin: f64,
    scale_max: f64,
    tick_step: f64,
    downlight_framing: bool,
) -> Document {
    let inset = margin.min(width.min(height) / 4.0) * 0.35;
    let label_step = if width.min(height) >= 720.0 { 15 } else { 30 };
    let max_angle = if downlight_framing { 90 } else { 180 };
    for gamma in (0..=max_angle).step_by(label_step) {
        for signed_gamma in if gamma == 0 || gamma == 180 {
            vec![f64::from(gamma)]
        } else {
            vec![-f64::from(gamma), f64::from(gamma)]
        } {
            let (edge_x, edge_y) = ray_to_bounds(
                center_x,
                center_y,
                signed_gamma,
                inset,
                inset,
                width - inset,
                height - inset,
            );
            let dx = center_x - edge_x;
            let dy = center_y - edge_y;
            let distance = dx.hypot(dy).max(1.0);
            doc = doc.add(
                Text::new(format!("{gamma}°"))
                    .set("x", edge_x + dx / distance * 10.0)
                    .set("y", edge_y + dy / distance * 10.0)
                    .set("font-family", "Arial, Helvetica, sans-serif")
                    .set("font-size", 12)
                    .set("text-anchor", "middle")
                    .set("dominant-baseline", "central")
                    .set("fill", "#555555"),
            );
        }
    }

    let mut value = tick_step;
    while value <= scale_max + tick_step * 1e-9 {
        let y = center_y + radius * value / scale_max;
        if y < height - inset - 18.0 {
            doc = doc.add(
                Text::new(format_number(value))
                    .set("x", center_x + 6.0)
                    .set("y", y - 4.0)
                    .set("font-family", "Arial, Helvetica, sans-serif")
                    .set("font-size", 12)
                    .set("fill", "#555555"),
            );
        }
        value += tick_step;
    }

    doc.add(
        Text::new("cd/klm")
            .set("x", inset + 4.0)
            .set("y", height - inset - 4.0)
            .set("font-family", "Arial, Helvetica, sans-serif")
            .set("font-size", 12)
            .set("fill", "#555555"),
    )
}

fn ray_to_bounds(
    center_x: f64,
    center_y: f64,
    theta: f64,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
) -> (f64, f64) {
    let radians = theta.to_radians();
    let dx = radians.sin();
    let dy = radians.cos();
    let tx = if dx > 1e-9 {
        (right - center_x) / dx
    } else if dx < -1e-9 {
        (left - center_x) / dx
    } else {
        f64::INFINITY
    };
    let ty = if dy > 1e-9 {
        (bottom - center_y) / dy
    } else if dy < -1e-9 {
        (top - center_y) / dy
    } else {
        f64::INFINITY
    };
    let distance = tx.min(ty);
    (center_x + dx * distance, center_y + dy * distance)
}

fn focused_scale(max_intensity: f64) -> (f64, f64) {
    let raw_step = max_intensity / 5.0;
    let exponent = raw_step.log10().floor();
    let base = 10.0_f64.powf(exponent);
    let normalized = raw_step / base;
    let nice = if normalized < 1.5 {
        1.0
    } else if normalized < 2.25 {
        2.0
    } else if normalized < 3.75 {
        2.5
    } else if normalized < 7.5 {
        5.0
    } else {
        10.0
    };
    let step = nice * base;
    let scale_max = (max_intensity / step).ceil() * step;
    (scale_max, step)
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
