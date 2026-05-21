//! Bundled font assets shared by the PDF and PNG generators.
//!
//! The generated SVG references `font-family: "Arial, Helvetica, sans-serif"`
//! and `font-weight: 700` for headers. We embed `DejaVu` Sans (Regular and
//! Bold) so the text-shaping step in usvg can resolve every glyph without
//! depending on host-installed fonts. The bundled family is wired up as the
//! generic `sans-serif` family at the call site via
//! `Database::set_sans_serif_family`.

/// `DejaVu` Sans Regular (version 2.37) — used for body text.
pub(crate) const DEJAVU_SANS_REGULAR: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");

/// `DejaVu` Sans Bold (version 2.37) — used for titles and section headers.
pub(crate) const DEJAVU_SANS_BOLD: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf");
