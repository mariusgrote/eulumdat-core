use svg2pdf::usvg::{Options, Tree};
use svg2pdf::{ConversionOptions, PageOptions};

use crate::generate::report_svg::ReportOptions;
use crate::{Eulumdat, EulumdatError};

impl Eulumdat {
    /// Renders a printable datasheet report as PDF bytes.
    pub fn to_report_pdf(&self, options: &ReportOptions) -> Result<Vec<u8>, EulumdatError> {
        let svg = self.to_report_svg(options)?;
        let tree = Tree::from_str(&svg, &Options::default()).map_err(|error| {
            EulumdatError::Generation(format!("failed to parse generated SVG: {error}"))
        })?;
        svg2pdf::to_pdf(&tree, ConversionOptions::default(), PageOptions::default())
            .map_err(|error| EulumdatError::Generation(format!("failed to generate PDF: {error}")))
    }
}
