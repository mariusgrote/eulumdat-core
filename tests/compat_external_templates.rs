#![allow(clippy::pedantic)]
#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]

use eulumdat_core::Eulumdat;

#[test]
fn parses_external_templates_if_configured() {
    let Some(dir) = std::env::var_os("QLUMEDIT_TEMPLATES_DIR") else {
        eprintln!("skipping: QLUMEDIT_TEMPLATES_DIR not set");
        return;
    };

    let mut parsed = 0usize;
    for entry in std::fs::read_dir(dir).expect("external template directory should be readable") {
        let path = entry.expect("directory entry should be readable").path();
        if path.extension().is_some_and(|extension| extension == "ldt") {
            Eulumdat::from_path(&path)
                .unwrap_or_else(|error| panic!("{} should parse: {error}", path.display()));
            parsed += 1;
        }
    }
    assert!(parsed > 0, "no .ldt files found in QLUMEDIT_TEMPLATES_DIR");
}
