//! `vissue hud` through a fake rofi, so CI never needs a compositor.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_vissue")
}

fn copy_fixture(dest: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixture_vault");
    copy_dir(&src, dest);
}

fn copy_dir(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dest.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), to).unwrap();
        }
    }
}

fn write_fake_rofi(dir: &Path, script: &str) -> PathBuf {
    let path = dir.join("fake-rofi");
    fs::write(&path, script).unwrap();
    let mut perm = fs::metadata(&path).unwrap().permissions();
    perm.set_mode(0o755);
    fs::set_permissions(&path, perm).unwrap();
    path
}

#[test]
fn hud_without_rofi_errors() {
    let out = Command::new(bin())
        .args(["--root", "/tmp/vissue-no-rofi", "hud"])
        .env_remove("VISSUE_ROFI")
        .env_remove("ROFI")
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("rofi is not installed"), "{err}");
}

#[test]
fn hud_claim_uses_alt_c_exit_code() {
    let tmp = tempfile::tempdir().unwrap();
    copy_fixture(tmp.path());
    let fake = write_fake_rofi(
        tmp.path(),
        r#"#!/bin/sh
# Claim an unclaimed ready row. atlas-1a2b is already held.
grep atlas-2c3d || head -n1
exit 10
"#,
    );
    let out = Command::new(bin())
        .args(["--root", tmp.path().to_str().unwrap(), "hud", "--mode", "ready"])
        .env("VISSUE_ROFI", &fake)
        .env("VISSUE_AGENT", "rofi-picker")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let show = Command::new(bin())
        .args([
            "--root",
            tmp.path().to_str().unwrap(),
            "show",
            "atlas-2c3d",
            "--json",
        ])
        .output()
        .unwrap();
    // First ready row in the fixture is atlas-1a2b (already claimed) or
    // atlas-2c3d. The fake claims whichever row is first. Check someone
    // is now held by rofi-picker.
    let claims = Command::new(bin())
        .args(["--root", tmp.path().to_str().unwrap(), "claims", "--json"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&claims.stdout);
    assert!(
        text.contains("rofi-picker"),
        "claimed row missing: {text}\nshow={}",
        String::from_utf8_lossy(&show.stdout)
    );
}
