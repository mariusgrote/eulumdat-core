# eulumdat-core

## Overview

`eulumdat-core` is a pure Rust library for EULUMDAT `.ldt` data.

It is implemented from public EULUMDAT format documentation and independently
authored tests. Third-party source files and template files are not
redistributed in this crate.

Milestone 1 is intentionally UI-free: no clipboard integration, rendering, FFI,
installer, or Qt application integration is included.

```rust
use eulumdat_core::Eulumdat;

fn main() -> Result<(), eulumdat_core::EulumdatError> {
    let text = std::fs::read_to_string("luminaire.ldt")?;
    let (mut ldt, warnings) = Eulumdat::parse(&text)?;
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
