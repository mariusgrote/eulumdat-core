//! UI-free EULUMDAT (`.ldt`) parsing, validation, serialization, and photometry.
//!
//! ```
//! use eulumdat_core::Eulumdat;
//!
//! # fn main() -> Result<(), eulumdat_core::EulumdatError> {
//! let text = "\
//! Independent synthetic fixture
//! 1
//! 1
//! 1
//! 0
//! 3
//! 90
//! SYN-REPORT-001
//! Synthetic luminaire
//! SYN-001
//! synthetic.ldt
//! 2026-05-13 test
//! 100
//! 100
//! 50
//! 80
//! 80
//! 0
//! 0
//! 0
//! 0
//! 50
//! 100
//! 1
//! 0
//! 1
//! 1
//! Synthetic LED
//! 1000
//! 4000K
//! 80
//! 10
//! 0
//! 0
//! 0
//! 0
//! 0
//! 0
//! 0
//! 0
//! 0
//! 0
//! 0
//! 0
//! 90
//! 180
//! 1000
//! 500
//! 10
//! ";
//! let (mut ldt, warnings) = Eulumdat::parse(&text)?;
//! println!("warnings: {}", warnings.len());
//! println!("output: {}", ldt.total_output());
//! println!("beam C0-C180: {:?}", ldt.beam_angle_c0_c180());
//! ldt.scale_to_100_percent();
//! let serialized = ldt.to_text();
//! # let _ = serialized;
//! # Ok(())
//! # }
//! ```
//!
//! Most fields are public for ergonomic inspection and editing. Manual mutation
//! can create invalid EULUMDAT state; call [`Eulumdat::validate`] before writing
//! modified data.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::if_not_else,
    clippy::manual_midpoint,
    clippy::many_single_char_names,
    clippy::match_same_arms,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::similar_names
)]

mod encoding;
mod error;
#[cfg(feature = "generate-svg")]
mod generate;
mod model;
mod parse;
mod photometry;
mod resample;
mod serialize;
mod table_parser;
mod validation;

pub use crate::encoding::TextEncoding;
pub use crate::error::{EulumdatError, ParseContext, ValidationWarning};
#[cfg(feature = "generate-svg")]
pub use crate::generate::{
    IntensityMode, PlanePair, PolarDiagramOptions, RasterBackground, RasterOptions, ReportOptions,
    ReportPageSize,
};
pub use crate::model::{
    Distribution, Eulumdat, LampSet, Symmetry, TypeIndicator, ValidationSettings,
};
pub use crate::table_parser::{TableDistribution, parse_table_text};
