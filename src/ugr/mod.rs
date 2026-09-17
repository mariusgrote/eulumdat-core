//! Unified Glare Rating (UGR) tables after the CIE tabular method.
//!
//! The UGR table follows CIE 117:1995 "Discomfort Glare in Interior Lighting"
//! and CIE 190:2010 "Calculation and Presentation of Unified Glare Rating
//! Tables for Indoor Lighting Luminaires".
//!
//! [`Eulumdat::ugr_table`](crate::Eulumdat::ugr_table) returns the table only
//! if the tabular method applies to the luminaire; otherwise it lists every
//! [`UgrBlocker`]. Values are not rounded.
//!
//! ```
//! use eulumdat_core::{Eulumdat, FluxBasis};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let text = "\
//! Synthetic UGR fixture
//! 1
//! 1
//! 1
//! 0
//! 19
//! 5
//! SYN-UGR-001
//! Synthetic downlight
//! SYN-UGR
//! ugr.ldt
//! 2026-09-17 test
//! 620
//! 620
//! 60
//! 600
//! 600
//! 0
//! 0
//! 0
//! 0
//! 100
//! 80
//! 1
//! 0
//! 1
//! 1
//! Synthetic LED
//! 3000
//! 4000K
//! 80
//! 25
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
//! 5
//! 10
//! 15
//! 20
//! 25
//! 30
//! 35
//! 40
//! 45
//! 50
//! 55
//! 60
//! 65
//! 70
//! 75
//! 80
//! 85
//! 90
//! 400
//! 398
//! 394
//! 386
//! 376
//! 363
//! 346
//! 328
//! 306
//! 283
//! 257
//! 229
//! 200
//! 169
//! 137
//! 104
//! 69
//! 35
//! 0
//! ";
//! let (ldt, _warnings) = Eulumdat::parse(text)?;
//! match ldt.ugr_table() {
//!     Ok(table) => {
//!         println!("lamp flux: {} lm", table.lamp_flux());
//!         for row in table.rows(FluxBasis::Normalized1000Lm) {
//!             let cells = row.crosswise.iter().chain(&row.endwise);
//!             let cells: Vec<String> = cells
//!                 .map(|cell| cell.map_or("-".to_string(), |value| format!("{value:.1}")))
//!                 .collect();
//!             println!("{}H x {}H: {}", row.room.x_h, row.room.y_h, cells.join(" "));
//!         }
//!         println!("data sheet: {:?}", table.data_sheet_value(FluxBasis::LampFlux));
//!     }
//!     Err(blockers) => {
//!         for blocker in blockers {
//!             println!("not applicable: {blocker}");
//!         }
//!     }
//! }
//! # assert!(ldt.ugr_table().is_ok());
//! # Ok(())
//! # }
//! ```
//!
//! The implementation is a port of the MIT-licensed Python packages
//! [eulumdat-ugr](https://github.com/123VincentB/eulumdat-ugr) and
//! [eulumdat-luminance](https://github.com/123VincentB/eulumdat-luminance)
//! (Copyright (c) 2026 123VincentB). Numerical choices such as interpolating
//! luminance rather than intensity follow those packages so results stay
//! comparable with their reference tables. Their copyright notice is in
//! `LICENSE`.

mod applicability;
mod background;
mod calculation;
mod geometry;
mod guth;
mod luminance;
mod result;
mod tables;

pub use applicability::UgrBlocker;
pub use result::{FluxBasis, UgrReflectances, UgrRoom, UgrRow, UgrTable, UgrView};
pub use tables::{UGR_REFLECTANCES, UGR_ROOMS};

#[cfg(test)]
#[path = "../../tests/common/ugr_reference.rs"]
mod ugr_reference;

#[cfg(test)]
mod table_tests;
#[cfg(test)]
mod tests;
