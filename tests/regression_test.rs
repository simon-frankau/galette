//
// regression_test.rs: Check that tool output is as expected.
//
// The standard way of doing Rust integration testing is to use a
// lib.rs file called by main.rs, but I really want to test all the
// way up to binary invocation to ensure missed coverage is minimal,
// so that's what we do here.
//

// TODO: Absolutely minimal-quality replacement for the shell script,
// since I want to rather rewrite how this works.

use std::fs::{self, create_dir_all, remove_dir_all, remove_file, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use test_bin::get_test_bin;

const TEST_TEMP_DIR: &str = "test_tmp";

// Yes, we re-open each time. Minimal change from the shell.
fn log_str(s: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("test_tmp/test.log")?;
    file.write_all(s.as_bytes())?;
    Ok(())
}

fn log_name(s: &str) -> Result<()> {
    log_str(&format!("=== {}\n", s))
}

#[test]
fn test_regression_old_school() -> Result<()> {
    if Path::new(TEST_TEMP_DIR).exists() {
        remove_dir_all(TEST_TEMP_DIR)?;
    }
    create_dir_all(TEST_TEMP_DIR)?;

    Command::new("sh")
        .args(["-c", &format!("cp testcases/*.pld {}", TEST_TEMP_DIR)])
        .spawn()?
        .wait()?;

    let mut names = Vec::new();
    for entry in fs::read_dir(TEST_TEMP_DIR)? {
        let name = entry?.file_name().to_str().unwrap().to_string();
        if name.ends_with(".pld") {
            names.push(name);
        }
    }
    names.sort();

    for name in names.iter() {
        log_name(&name)?;

        let log_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("test_tmp/test.log")?;
        let log_file2 = log_file.try_clone().unwrap();

        get_test_bin("galette")
            .arg(&name)
            .current_dir(TEST_TEMP_DIR)
            .stdout(log_file)
            .stderr(log_file2)
            .spawn()?
            .wait()?;

        remove_file(&format!("{}/{}", TEST_TEMP_DIR, name))?;
    }

    let diff_res = Command::new("diff")
        .args(["-ru", "baseline", "test_tmp"])
        .status()?;

    assert!(diff_res.success(), "Output generation differs");

    Ok(())
}

fn ensure_dir_exists(name: &str) -> Result<()> {
    if Path::new(name).exists() {
        remove_dir_all(name)?;
    }
    create_dir_all(name)?;
    Ok(())
}

fn get_plds(dir: &str) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let name = entry?.file_name().to_str().unwrap().to_string();
        if name.ends_with(".pld") {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn check_invocation_succeeded(name: &str, res: std::process::Output) {
    assert!(res.status.success(), "'{:?}' did not succeed", name);
    assert_eq!(
        res.stdout.len(),
        0,
        "'{:?}' produced unexpected output to stdout: {:?}",
        name,
        std::str::from_utf8(&res.stdout).unwrap()
    );
    assert_eq!(
        res.stderr.len(),
        0,
        "'{:?}' produced unexpected output to stderr: {:?}",
        name,
        std::str::from_utf8(&res.stderr).unwrap()
    );
}

fn check_output_matches(before_dir: &str, after_dir: &str) -> Result<()> {
    let diff_res = Command::new("diff")
        .args(["-ru", before_dir, after_dir])
        .status()?;
    assert!(
        diff_res.success(),
        "Output generation differs (run with --nocapture for details)"
    );
    Ok(())
}

#[test]
fn test_successful_generation() -> Result<()> {
    ensure_dir_exists("test_temp_success")?;

    for name in get_plds("testcases_success")?.iter() {
        std::fs::copy(
            format!("testcases_success/{}", name),
            format!("test_temp_success/{}", name),
        )?;

        let results = get_test_bin("galette")
            .current_dir("test_temp_success")
            .arg(name)
            .output()?;
        check_invocation_succeeded(name, results);
    }

    check_output_matches("testcases_success", "test_temp_success")?;
    Ok(())
}

#[test]
fn test_security_bit() -> Result<()> {
    ensure_dir_exists("test_temp_security")?;
    std::fs::copy(
        "testcases_security/security_bit.pld",
        "test_temp_security/security_bit.pld",
    )?;

    let results = get_test_bin("galette")
        .current_dir("test_temp_security")
        .args(["-s", "security_bit.pld"])
        .output()?;
    check_invocation_succeeded("security.pld", results);

    check_output_matches("testcases_security", "test_temp_security")?;
    Ok(())
}
