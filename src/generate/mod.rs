//! Generation helpers for photometric diagrams and reports.

#[cfg(feature = "generate-pdf")]
mod pdf;
#[cfg(feature = "generate-png")]
mod png;
#[cfg(feature = "generate-svg")]
mod polar_svg;
#[cfg(feature = "generate-svg")]
mod report_svg;

#[cfg(feature = "generate-png")]
pub use png::{RasterBackground, RasterOptions};
#[cfg(feature = "generate-svg")]
pub use polar_svg::{IntensityMode, PlanePair, PolarDiagramOptions};
#[cfg(not(feature = "generate-png"))]
pub use report_svg::{RasterBackground, RasterOptions};
#[cfg(feature = "generate-svg")]
pub use report_svg::{ReportOptions, ReportPageSize};
