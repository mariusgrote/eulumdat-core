#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![cfg(feature = "generate-png")]

mod common;

use eulumdat_core::{PolarDiagramOptions, RasterBackground, RasterOptions, Symmetry};

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
