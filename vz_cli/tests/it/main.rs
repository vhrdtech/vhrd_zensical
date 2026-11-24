//! The `vz` binary as a person runs it.

use std::process::Command;

/// `vz` with the outer cargo's `CARGO_*` variables removed, so it sees the environment a person's shell gives
fn vz() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vz"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("CARGO_") {
            cmd.env_remove(key);
        }
    }
    cmd
}

#[test]
fn version_names_the_cargo_version() {
    let out = vz().arg("--version").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.starts_with(&format!("vz {} (", env!("CARGO_PKG_VERSION"))),
        "{stdout}"
    );
}

#[test]
fn no_command_prints_help_and_fails() {
    let out = vz().arg("--color").arg("never").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8(out.stderr).unwrap().contains("Usage: vz"));
}

#[test]
fn completion_registers_for_each_shell() {
    for shell in ["bash", "zsh", "fish"] {
        let out = vz().env("COMPLETE", shell).output().unwrap();
        assert!(out.status.success(), "{shell}");
        assert!(!out.stdout.is_empty(), "{shell}: no registration");
    }
}
