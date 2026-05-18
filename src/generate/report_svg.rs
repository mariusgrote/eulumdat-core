use svg::Document;
use svg::node::element::{Group, Rectangle, Text};

use crate::generate::polar_svg::{PolarDiagramOptions, format_number};
use crate::{Eulumdat, EulumdatError};

/// Options used when rendering a printable datasheet report.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportOptions {
    /// Physical page size for the generated report.
    pub page_size: ReportPageSize,
    /// Optional report title. Defaults to the luminaire name.
    pub title: Option<String>,
    /// Polar diagram options used inside the report.
    pub polar: PolarDiagramOptions,
    /// Whether to include a compact intensity matrix summary.
    pub include_intensity_table_summary: bool,
}

/// Supported PDF/SVG report page sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportPageSize {
    /// ISO A4 portrait, represented in PDF points.
    A4Portrait,
}

/// Options used when rasterizing generated SVG.
#[cfg(not(feature = "generate-png"))]
#[derive(Debug, Clone, PartialEq)]
pub struct RasterOptions {
    /// Output image width in pixels.
    pub width: u32,
    /// Output image height in pixels.
    pub height: u32,
    /// Raster background fill.
    pub background: RasterBackground,
}

/// Raster background fill mode.
#[cfg(not(feature = "generate-png"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterBackground {
    /// Preserve transparency.
    Transparent,
    /// Fill the pixmap with white before rendering.
    White,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            page_size: ReportPageSize::A4Portrait,
            title: None,
            polar: PolarDiagramOptions::default(),
            include_intensity_table_summary: true,
        }
    }
}

#[cfg(not(feature = "generate-png"))]
impl Default for RasterOptions {
    fn default() -> Self {
        Self {
            width: 800,
            height: 800,
            background: RasterBackground::White,
        }
    }
}

impl ReportPageSize {
    fn dimensions(self) -> (u32, u32) {
        match self {
            Self::A4Portrait => (595, 842),
        }
    }
}

impl Eulumdat {
    /// Renders a printable datasheet report as SVG.
    pub fn to_report_svg(&self, options: &ReportOptions) -> Result<String, EulumdatError> {
        let (width, height) = options.page_size.dimensions();
        let title = options
            .title
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or(&self.luminaire_name);

        let mut polar_options = options.polar.clone();
        polar_options.width = 515;
        polar_options.height = 370;
        polar_options.margin = 42.0;
        polar_options.title = None;
        let rendered_polar = self.render_polar_svg(&polar_options)?;

        let mut doc = Document::new()
            .set("xmlns", "http://www.w3.org/2000/svg")
            .set("viewBox", (0, 0, width, height))
            .set("width", width)
            .set("height", height)
            .set("role", "document")
            .add(
                Rectangle::new()
                    .set("x", 0)
                    .set("y", 0)
                    .set("width", width)
                    .set("height", height)
                    .set("fill", "#ffffff"),
            );

        doc = doc.add(
            Text::new(title.to_string())
                .set("x", 40)
                .set("y", 42)
                .set("font-family", "Arial, Helvetica, sans-serif")
                .set("font-size", 22)
                .set("font-weight", "700")
                .set("fill", "#111111"),
        );
        doc = doc.add(
            Text::new(format!(
                "{} / {}",
                fallback(&self.luminaire_number),
                fallback(&self.measurement_report_number)
            ))
            .set("x", 40)
            .set("y", 64)
            .set("font-family", "Arial, Helvetica, sans-serif")
            .set("font-size", 11)
            .set("fill", "#555555"),
        );

        let mut y = 104.0;
        doc = add_section(doc, "File metadata", 40.0, y);
        y += 20.0;
        for (label, value) in [
            ("File", fallback(&self.file_name)),
            ("Date/user", fallback(&self.date_user)),
            ("Type", format!("{:?}", self.type_indicator)),
            ("Symmetry", format!("{:?}", self.symmetry)),
        ] {
            doc = add_row(doc, label, &value, 40.0, y);
            y += 17.0;
        }

        let mut right_y = 104.0;
        doc = add_section(doc, "Photometry", 310.0, right_y);
        right_y += 20.0;
        for (label, value) in [
            (
                "Downward flux recorded",
                format!("{} %", format_number(self.downward_flux_fraction)),
            ),
            (
                "Downward flux calculated",
                format!(
                    "{} %",
                    format_number(self.calculated_downward_flux_fraction())
                ),
            ),
            (
                "Light output ratio",
                format!("{} %", format_number(self.light_output_ratio)),
            ),
            ("Total output", format_number(self.total_output())),
            ("Beam C0-C180", option_degrees(self.beam_angle_c0_c180())),
            ("Beam C90-C270", option_degrees(self.beam_angle_c90_c270())),
            ("Field C0-C180", option_degrees(self.field_angle_c0_c180())),
            (
                "Field C90-C270",
                option_degrees(self.field_angle_c90_c270()),
            ),
        ] {
            doc = add_row(doc, label, &value, 310.0, right_y);
            right_y += 17.0;
        }

        y += 14.0;
        doc = add_section(doc, "Dimensions", 40.0, y);
        y += 20.0;
        for (label, value) in [
            (
                "Luminaire L/W/H",
                dimensions(
                    self.luminaire_length,
                    self.luminaire_width,
                    self.luminaire_height,
                ),
            ),
            (
                "Luminous area L/W",
                format!(
                    "{} / {} mm",
                    format_number(self.luminous_area_length),
                    format_number(self.luminous_area_width)
                ),
            ),
            (
                "Luminous area H C0/C90/C180/C270",
                format!(
                    "{} / {} / {} / {} mm",
                    format_number(self.luminous_area_height_c0),
                    format_number(self.luminous_area_height_c90),
                    format_number(self.luminous_area_height_c180),
                    format_number(self.luminous_area_height_c270)
                ),
            ),
        ] {
            doc = add_row(doc, label, &value, 40.0, y);
            y += 17.0;
        }

        y += 14.0;
        doc = add_section(doc, "Lamps", 40.0, y);
        y += 20.0;
        for lamp in &self.lamps {
            let value = format!(
                "{} x {}, {} lm, {}, CRI {}, {} W",
                lamp.lamp_count,
                fallback(&lamp.lamp_type),
                format_number(lamp.total_luminous_flux),
                fallback(&lamp.color_temperature),
                fallback(&lamp.color_rendering_index),
                format_number(lamp.wattage_including_ballast)
            );
            doc = add_row(doc, "Lamp set", &value, 40.0, y);
            y += 17.0;
        }

        if options.include_intensity_table_summary {
            y += 14.0;
            doc = add_section(doc, "Intensity table", 40.0, y);
            y += 20.0;
            doc = add_row(
                doc,
                "Shape",
                &format!(
                    "{} C-planes, {} gamma angles, {} stored rows",
                    self.c_plane_count(),
                    self.gamma_count(),
                    self.intensities.len()
                ),
                40.0,
                y,
            );
        }

        let polar_group = Group::new()
            .set("transform", "translate(40 438)")
            .add(rendered_polar.document);
        doc = doc.add(
            Text::new("Polar diagram".to_string())
                .set("x", 40)
                .set("y", 418)
                .set("font-family", "Arial, Helvetica, sans-serif")
                .set("font-size", 14)
                .set("font-weight", "700")
                .set("fill", "#222222"),
        );
        doc = doc.add(polar_group);

        if !rendered_polar.notes.is_empty() {
            doc = doc.add(
                Text::new(rendered_polar.notes.join("; "))
                    .set("x", 40)
                    .set("y", 824)
                    .set("font-family", "Arial, Helvetica, sans-serif")
                    .set("font-size", 9)
                    .set("fill", "#777777"),
            );
        }

        Ok(doc.to_string())
    }
}

fn add_section(mut doc: Document, label: &str, x: f64, y: f64) -> Document {
    doc = doc.add(
        Text::new(label.to_string())
            .set("x", x)
            .set("y", y)
            .set("font-family", "Arial, Helvetica, sans-serif")
            .set("font-size", 14)
            .set("font-weight", "700")
            .set("fill", "#222222"),
    );
    doc
}

fn add_row(mut doc: Document, label: &str, value: &str, x: f64, y: f64) -> Document {
    doc = doc
        .add(
            Text::new(label.to_string())
                .set("x", x)
                .set("y", y)
                .set("font-family", "Arial, Helvetica, sans-serif")
                .set("font-size", 10)
                .set("fill", "#666666"),
        )
        .add(
            Text::new(value.to_string())
                .set("x", x + 105.0)
                .set("y", y)
                .set("font-family", "Arial, Helvetica, sans-serif")
                .set("font-size", 10)
                .set("fill", "#111111"),
        );
    doc
}

fn fallback(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn dimensions(length: f64, width: f64, height: f64) -> String {
    format!(
        "{} / {} / {} mm",
        format_number(length),
        format_number(width),
        format_number(height)
    )
}

fn option_degrees(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_string(),
        |value| format!("{}°", format_number(value)),
    )
}
