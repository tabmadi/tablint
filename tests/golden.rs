//! Runs tablint on each fixture repository and compares its output with `<fixture>.expected`. Run with
//! `TABLINT_BLESS=1` to write the expected output after a deliberate change, then review the diff.

use std::path::Path;
use std::process::Command;

#[test]
fn fixtures() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("the fixtures directory exists") {
        let root = entry.expect("a readable entry").path();
        if !root.is_dir() {
            continue;
        }
        let out = Command::new(env!("CARGO_BIN_EXE_tablint"))
            .arg("check")
            .arg(&root)
            .output()
            .expect("tablint runs");
        let got = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let expected = root.with_extension("expected");
        if std::env::var_os("TABLINT_BLESS").is_some() {
            std::fs::write(&expected, &got).expect("the expected file is writable");
        }
        let want = std::fs::read_to_string(&expected).expect("each fixture has an expected file");
        assert_eq!(got, want, "the output for {} changed", root.display());
        checked += 1;
    }
    assert!(checked > 0, "no fixture ran");
}
