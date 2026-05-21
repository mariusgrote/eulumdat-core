#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![cfg(feature = "generate-png")]

mod common;

use eulumdat_core::{PolarDiagramOptions, RasterBackground, RasterOptions, Symmetry};
use tiny_skia::Pixmap;

#[test]
fn polar_png_has_png_magic_header() {
    let model = common::synthetic_model(
        Symmetry::C0C180,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    let png = model
        .to_polar_png(&PolarDiagramOptions::default(), &RasterOptions::default())
        .expect("PNG should render");

    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn polar_png_supports_transparent_background() {
    let model = common::synthetic_model(
        Symmetry::C0C180,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    let raster = RasterOptions {
        background: RasterBackground::Transparent,
        ..RasterOptions::default()
    };
    let png = model
        .to_polar_png(&PolarDiagramOptions::default(), &raster)
        .expect("transparent PNG should render");

    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn polar_png_renders_axis_label_text() {
    let model = common::synthetic_model(
        Symmetry::C0C180,
        eulumdat_core::TypeIndicator::PointSourceWithSymmetry,
    );
    let png = model
        .to_polar_png(&PolarDiagramOptions::default(), &RasterOptions::default())
        .expect("PNG should render");

    let pixmap = Pixmap::decode_png(&png).expect("encoded PNG should decode");
    let dark_gray = count_text_like_pixels(&pixmap);

    // Axis labels use #555555 / #666666 and the legend uses #222222 — without
    // a fontdb the entire diagram has no dark-gray pixels (background is
    // white, grid is #d8d8d8, curves are saturated colors). A floor of 200
    // text-like pixels is well above any anti-aliasing on the grid lines.
    assert!(
        dark_gray > 200,
        "expected the rendered polar diagram to contain dark-gray text pixels \
         (axis labels, legend); got {dark_gray} which suggests text was dropped"
    );
}

fn count_text_like_pixels(pixmap: &Pixmap) -> usize {
    pixmap
        .pixels()
        .iter()
        .filter(|pixel| {
            if pixel.alpha() == 0 {
                return false;
            }
            let r = pixel.red();
            let g = pixel.green();
            let b = pixel.blue();
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            // Dark and near-grayscale: excludes white background, light-gray
            // grid (#d8d8d8), and saturated curve colors.
            max <= 150 && max.saturating_sub(min) <= 20
        })
        .count()
}
