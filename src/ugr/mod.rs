//! Unified Glare Rating (UGR) building blocks for the CIE tabular method.
//!
//! The UGR table follows CIE 117:1995 "Discomfort Glare in Interior Lighting"
//! and CIE 190:2010 "Calculation and Presentation of Unified Glare Rating
//! Tables for Indoor Lighting Luminaires".
//!
//! The implementation is a port of the MIT-licensed Python packages
//! [eulumdat-ugr](https://github.com/123VincentB/eulumdat-ugr) and
//! [eulumdat-luminance](https://github.com/123VincentB/eulumdat-luminance)
//! (Copyright (c) 2026 123VincentB). Numerical choices such as interpolating
//! luminance rather than intensity follow those packages so results stay
//! comparable with their reference tables. Their copyright notice is in
//! `LICENSE`.

mod background;
mod calculation;
mod geometry;
mod guth;
mod luminance;
mod tables;

#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "the UGR table is wired into the public API in a later phase"
    )
)]
pub(crate) use calculation::UgrTable;
#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "the UGR table is wired into the public API in a later phase"
    )
)]
pub(crate) use tables::UGR_REFLECTANCES;
pub(crate) use tables::UGR_ROOMS;

#[cfg(test)]
mod table_tests;
#[cfg(test)]
mod tests;
