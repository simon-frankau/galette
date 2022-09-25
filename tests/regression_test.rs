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

use std::env::set_current_dir;
use std::fs::{create_dir_all, remove_dir_all, remove_file, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use glob::glob;
use test_bin::get_test_bin;

const TEST_TEMP_DIR: &str = "test_tmp2/";

// Yes, we re-open each time. Minimal change from the shell.
fn log_str(s: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("test.log")?;
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

    set_current_dir(TEST_TEMP_DIR)?;

    // Special pass for security bit flag test:
    Command::new("sh")
        .args([
            "-c",
            &format!("cp GAL16V8_combinatorial.pld security_bit.pld"),
        ])
        .spawn()?
        .wait()?;

    log_name("security_bit.pld")?;

    get_test_bin("galette")
        .arg("-s")
        .arg("security_bit.pld")
        .spawn()?
        .wait()?;

    for file in glob("*.pld").unwrap() {
        let file = file.unwrap();
        log_name(&file.as_os_str().to_str().unwrap())?;

	let log_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("test.log")?;
	let log_file2 = log_file.try_clone().unwrap();
	
        get_test_bin("galette").arg(&file).stdout(log_file).stderr(log_file2).spawn()?.wait()?;
        remove_file(&file)?;
    }

    set_current_dir("..")?;

    let diff_res = Command::new("diff")
        .args(["-ru", "baseline", "test_tmp2"])
        .status()?;

    assert!(diff_res.success(), "Output generation differs");

    Ok(())
}
