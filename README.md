# eulumdat-core

## Overview

`eulumdat-core` is a pure Rust library for EULUMDAT `.ldt` data.

The core is implemented from public EULUMDAT format documentation and
independently authored tests. The UGR module is ported from MIT-licensed Python
packages; see [Provenance](#provenance).

Milestone 1 is intentionally UI-free: no clipboard integration, rendering, FFI,
installer, or Qt application integration is included.

```rust
use eulumdat_core::Eulumdat;

fn main() -> Result<(), eulumdat_core::EulumdatError> {
    let (mut ldt, warnings) = Eulumdat::from_path("luminaire.ldt")?;
    println!("warnings: {}", warnings.len());
    println!("total output: {}", ldt.total_output());
    println!("beam C0-C180: {:?}", ldt.beam_angle_c0_c180());

    ldt.scale_to_100_percent();
    ldt.write_path("scaled.ldt")?;
    Ok(())
}
```

Most model fields are public for inspection and editing. Manual mutation can
create invalid states; call `validate()` before serializing modified data.

`.ldt` files are commonly UTF-8, Windows-1252, or Latin-1-like text. Byte and
file APIs decode UTF-8 first and fall back to Windows-1252 for legacy files.
String APIs such as `Eulumdat::parse` assume the caller has already decoded the
text.

## Provenance

The crate is intended to be implemented from public EULUMDAT format
documentation, standard photometric formulas, standard numerical methods, and
independently authored tests.

Reference material for the file format:

- AGI/Photometric Toolbox EULUMDAT format description:
  <https://docs.agi32.com/PhotometricToolbox/Content/Open_Tool/eulumdat_file_format.htm>
- DIALux EULUMDAT format description:
  <https://evo.support-en.dial.de/support/solutions/articles/9000074164-description-of-the-eulumdat-format>
- Paul Bourke EULUMDAT notes:
  <https://paulbourke.net/dataformats/ldt/>
- QLumEdit EULUMDAT format description:
  <https://github.com/cagrin/qlumedit>

This repository does not include QLumEdit source files or template files.
Optional interoperability tests can be run against a user-provided local
template directory via `QLUMEDIT_TEMPLATES_DIR`; those files are not copied into
or redistributed with this crate.

The calculated direct-ratio helper is intentionally not included until its
coefficient tables can be tied to an independent non-GPL source.

### UGR calculation

The UGR (Unified Glare Rating) code in `src/ugr/` follows CIE 117:1995 and
CIE 190:2010. It is a Rust port of the logic in these MIT-licensed Python
packages by 123VincentB:

- [eulumdat-ugr](https://github.com/123VincentB/eulumdat-ugr): UGR table
  calculation
- [eulumdat-luminance](https://github.com/123VincentB/eulumdat-luminance):
  luminance and projected luminous area

The UGR test fixtures in `tests/fixtures/ugr/` come from eulumdat-ugr. They
include sample `.ldt` files and reference tables from Relux, DIALux and
CIE 190:2010. Reference values were also generated with eulumdat-ugr,
eulumdat-luminance and [eulumdat-py](https://github.com/123VincentB/pyldt).
These packages use the same MIT license as this crate; their copyright notice
is in [LICENSE](LICENSE). `tests/fixtures/ugr/README.md` describes the fixtures
in detail.
