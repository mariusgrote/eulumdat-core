use tiny_skia::{Color, Pixmap, Transform};
use usvg::{Options, Tree};

use crate::generate::fonts::{DEJAVU_SANS_BOLD, DEJAVU_SANS_REGULAR};
use crate::generate::polar_svg::PolarDiagramOptions;
use crate::{Eulumdat, EulumdatError};

/// Options used when rasterizing generated SVG.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterBackground {
    /// Preserve transparency.
    Transparent,
    /// Fill the pixmap with white before rendering.
    White,
}

impl Default for RasterOptions {
    fn default() -> Self {
        Self {
            width: 800,
            height: 800,
            background: RasterBackground::White,
        }
    }
}

impl Eulumdat {
    /// Renders the model's selected polar diagram as encoded PNG bytes.
    pub fn to_polar_png(
        &self,
        polar: &PolarDiagramOptions,
        raster: &RasterOptions,
    ) -> Result<Vec<u8>, EulumdatError> {
        let svg = self.to_polar_svg(polar)?;
        let mut svg_options = Options::default();
        {
            let db = svg_options.fontdb_mut();
            db.load_font_data(DEJAVU_SANS_REGULAR.to_vec());
            db.load_font_data(DEJAVU_SANS_BOLD.to_vec());
            db.set_sans_serif_family("DejaVu Sans");
        }
        let tree = Tree::from_str(&svg, &svg_options).map_err(|error| {
            EulumdatError::Generation(format!("failed to parse generated SVG: {error}"))
        })?;
        let mut pixmap = Pixmap::new(raster.width, raster.height)
            .ok_or_else(|| EulumdatError::Generation("invalid raster dimensions".to_string()))?;
        if raster.background == RasterBackground::White {
            pixmap.fill(Color::WHITE);
        }

        let size = tree.size();
        let sx = f64::from(raster.width) / f64::from(size.width());
        let sy = f64::from(raster.height) / f64::from(size.height());
        let transform = Transform::from_scale(sx as f32, sy as f32);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        pixmap
            .encode_png()
            .map_err(|error| EulumdatError::Generation(format!("failed to encode PNG: {error}")))
    }
}
