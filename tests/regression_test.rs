//
// regression_test.rs: Check that tool output is as expected.
//
// The standard way of doing Rust integration testing is to use a
// lib.rs file called by main.rs, but I really want to test all the
// way up to binary invocation to ensure missed coverage is minimal,
// so that's what we do here.
//

use std::collections::{HashMap, HashSet};
use std::fs::{self, create_dir_all, read_to_string, remove_dir_all};
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Result};
use test_bin::get_test_bin;

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
    assert!(
        res.stdout.is_empty(),
        "'{:?}' produced unexpected output to stdout: {:?}",
        name,
        std::str::from_utf8(&res.stdout).unwrap()
    );
    assert!(
        res.stderr.is_empty(),
        "'{:?}' produced unexpected output to stderr: {:?}",
        name,
        std::str::from_utf8(&res.stderr).unwrap()
    );
    // Do this last, so we can see useful error output if it exists.
    assert!(res.status.success(), "'{:?}' did not succeed", name);
}

fn check_invocation_failed(name: &str, messages: &HashMap<&str, &str>, res: std::process::Output) {
    assert!(
        !res.status.success(),
        "'{:?}' succeeded when failure was expected",
        name
    );
    assert!(
        res.stdout.is_empty(),
        "'{:?}' produced unexpected output to stdout: {:?}",
        name,
        std::str::from_utf8(&res.stdout).unwrap()
    );
    assert_eq!(
        std::str::from_utf8(&res.stderr).unwrap(),
        format!(
            "{}: {}",
            name,
            *messages
                .get(name)
                .expect(&format!("No known error message for '{}'", name))
        ),
        "'{:?}' produced unexpected output to stderr",
        name
    );
}

// Ensure the list of files present in one directory are all present in the other.
fn ensure_contains(
    containing_dir: &str,
    containing_names: &HashSet<String>,
    contained_dir: &str,
    contained_names: &HashSet<String>,
) -> Result<()> {
    if contained_names.is_subset(containing_names) {
        return Ok(());
    }

    let mut missing_names = contained_names
        .difference(&containing_names)
        .collect::<Vec<_>>();
    missing_names.sort();

    bail!(
        "Missing expected files, present in '{}', but not in '{}': {:?}",
        contained_dir,
        containing_dir,
        missing_names
    );
}

fn check_output_matches(before_dir: &str, after_dir: &str) -> Result<()> {
    // Get lists of files.
    let befores = fs::read_dir(before_dir)?
        .map(|name| name.map(|name| name.file_name().to_str().unwrap().to_string()))
        .collect::<std::result::Result<HashSet<String>, _>>()?;
    let afters = fs::read_dir(after_dir)?
        .map(|name| name.map(|name| name.file_name().to_str().unwrap().to_string()))
        .collect::<std::result::Result<HashSet<String>, _>>()?;

    // Check they match.
    ensure_contains(before_dir, &befores, after_dir, &afters)?;
    ensure_contains(after_dir, &afters, before_dir, &befores)?;

    // Check the contents match.
    for file in befores.iter() {
        let before_name = format!("{}/{}", before_dir, file);
        let after_name = format!("{}/{}", after_dir, file);
        let before_data = read_to_string(&before_name)?;
        let after_data = read_to_string(&after_name)?;
        if before_data != after_data {
            // Try to run diff on platforms that support it.
            let _ = Command::new("diff")
                .args(["-u", "--", &before_name, &after_name])
                .status();

            bail!(
                "Output generation differs between '{}' and '{}'.",
                before_name,
                after_name
            );
        }
    }

    Ok(())
}

#[test]
fn test_successful_generation() -> Result<()> {
    ensure_dir_exists("test_temp_success")?;

    for name in get_plds("testcases/success")?.iter() {
        std::fs::copy(
            format!("testcases/success/{}", name),
            format!("test_temp_success/{}", name),
        )?;

        let results = get_test_bin("galette")
            .current_dir("test_temp_success")
            .arg(name)
            .output()?;
        check_invocation_succeeded(name, results);
    }

    check_output_matches("testcases/success", "test_temp_success")?;

    remove_dir_all("test_temp_success")?;
    Ok(())
}

#[test]
fn test_security_bit() -> Result<()> {
    ensure_dir_exists("test_temp_security")?;

    std::fs::copy(
        "testcases/security/security_bit.pld",
        "test_temp_security/security_bit.pld",
    )?;

    let results = get_test_bin("galette")
        .current_dir("test_temp_security")
        .args(["-s", "security_bit.pld"])
        .output()?;
    check_invocation_succeeded("security.pld", results);

    check_output_matches("testcases/security", "test_temp_security")?;

    remove_dir_all("test_temp_security")?;
    Ok(())
}

const FAILURE_MESSAGES: [(&str, &str); 82] = [
    ("GAL16V8_badname.pld", "Error in line 1: unexpected GAL type found: 'GAL16V8x'\n"),
    ("GAL16V8_complex_12.pld", "Error in line 9: pin 12 can't be used as input in complex mode\n"),
    ("GAL16V8_complex_19.pld", "Error in line 9: pin 19 can't be used as input in complex mode\n"),
    ("GAL16V8_reg_1.pld", "Error in line 7: pin 1 is reserved for 'Clock' in registered mode\n"),
    ("GAL16V8_reg_11.pld", "Error in line 7: pin 11 is reserved for '/OE' in registered mode\n"),
    ("GAL20RA10_badname.pld", "Error in line 1: unexpected GAL type found: 'GAL20RA10x'\n"),
    ("GAL20RA10_pin1.pld", "Error in line 7: pin 1 is reserved for '/PL' on GAL20RA10 devices and can't be used in equations\n"),
    ("GAL20RA10_pin13.pld", "Error in line 7: pin 13 is reserved for '/OE' on GAL20RA10 devices and can't be used in equations\n"),
    ("GAL20V8_badname.pld", "Error in line 1: unexpected GAL type found: 'GAL20V8x'\n"),
    ("GAL20V8_complex_15.pld", "Error in line 9: pin 15 can't be used as input in complex mode\n"),
    ("GAL20V8_complex_22.pld", "Error in line 9: pin 22 can't be used as input in complex mode\n"),
    ("GAL20V8_complex_in.pld", "Error in line 5: pinname I8 is defined twice\n"),
    ("GAL20V8_reg_1.pld", "Error in line 7: pin 1 is reserved for 'Clock' in registered mode\n"),
    ("GAL20V8_reg_13.pld", "Error in line 7: pin 13 is reserved for '/OE' in registered mode\n"),
    ("GAL22V10_badname.pld", "Error in line 1: unexpected GAL type found: 'GAL22V10x'\n"),
    ("arbad.pld", "Error in line 5: GAL22V10: AR is not allowed as pinname\n"),
    ("badarext.pld", "Error in line 23: no suffix is allowed for AR\n"),
    ("badarusage.pld", "Error in line 21: use of AR is not allowed in equations\n"),
    ("badclk.pld", "Error in line 7: .CLK is not allowed when this type of GAL is used\n"),
    ("badgnd.pld", "Error in line 4: pin 8 cannot be named GND, because the name is reserved for pin 10\n"),
    ("badname.pld", "Error in line 1: unexpected GAL type found: 'GAL42V13'\n"),
    ("badpinstart.pld", "Error in line 4: expected pin, found other token\n"),
    ("badprst.pld", "Error in line 7: .APRST is not allowed when this type of GAL is used\n"),
    ("badrst.pld", "Error in line 7: .ARST is not allowed when this type of GAL is used\n"),
    ("badspext.pld", "Error in line 23: no suffix is allowed for SP\n"),
    ("badspusage.pld", "Error in line 21: use of SP is not allowed in equations\n"),
    ("badvcc.pld", "Error in line 4: pin 8 cannot be named VCC, because the name is reserved for pin 20\n"),
    ("continuation_bad.pld", "Error in line 12: expected pin, found other token\n"),
    ("inputonly.pld", "Error in line 7: this pin can't be used as output\n"),
    ("logicgnd.pld", "Error in line 7: use of VCC and GND is not allowed in equations\n"),
    ("logicvcc.pld", "Error in line 7: use of VCC and GND is not allowed in equations\n"),
    ("longext.pld", "Error in line 7: unknown suffix found: 'TOOLONGEXTENSION'\n"),
    ("multiar.pld", "Error in line 23: only one product term allowed (no OR)\n"),
    ("multiclk.pld", "Error in line 22: only one product term allowed (no OR)\n"),
    ("multiena.pld", "Error in line 15: only one product term allowed (no OR)\n"),
    ("multiprst.pld", "Error in line 22: only one product term allowed (no OR)\n"),
    ("multirst.pld", "Error in line 22: only one product term allowed (no OR)\n"),
    ("multisp.pld", "Error in line 23: only one product term allowed (no OR)\n"),
    ("nclhs.pld", "Error in line 17: NC (Not Connected) is not allowed in logic equations\n"),
    ("ncpin.pld", "Error in line 9: NC (Not Connected) is not allowed in logic equations\n"),
    ("negaprst.pld", "Error in line 25: negation of .APRST is not allowed\n"),
    ("negar.pld", "Error in line 23: negation of AR is not allowed\n"),
    ("negarst.pld", "Error in line 24: negation of .ARST is not allowed\n"),
    ("negclk.pld", "Error in line 8: negation of .CLK is not allowed\n"),
    ("negena.pld", "Error in line 17: negation of .E is not allowed\n"),
    ("neggnd.pld", "Error in line 7: GND cannot be negated, use VCC instead of /GND\n"),
    ("negsp.pld", "Error in line 25: negation of SP is not allowed\n"),
    ("negvcc.pld", "Error in line 7: VCC cannot be negated, use GND instead of /VCC\n"),
    ("noclk.pld", "Error in line 7: missing clock definition (.CLK) of registered output\n"),
    ("noequals.pld", "Error in line 7: unexpected character in input: '?'\n"),
    ("nognd.pld", "Error in line 4: pin 10 must be named GND\n"),
    ("norhs.pld", "Error in line 7: expected right-hand side of equation, found end of file\n"),
    ("norhs2.pld", "Error in line 7: expected right-hand side of equation, found end of file\n"),
    ("norhs3.pld", "Error in line 7: expected pin name, found end of line\n"),
    ("novcc.pld", "Error in line 5: pin 20 must be named VCC\n"),
    ("oneline.pld", "Error in line 1: expected signature, found end of file\n"),
    ("onlyclk.pld", "Error in line 10: the output must be defined to use .CLK\n"),
    ("onlyenable.pld", "Error in line 10: the output must be defined to use .E\n"),
    ("onlyprst.pld", "Error in line 10: the output must be defined to use .APRST\n"),
    ("onlyrst.pld", "Error in line 10: the output must be defined to use .ARST\n"),
    ("pinbadneg.pld", "Error in line 4: pin name expected after '/', found non-alphabetic character ' '\n"),
    ("pinrepeated.pld", "Error in line 4: pinname I5 is defined twice\n"),
    ("plaintri.pld", "Error in line 8: tristate control without previous '.T'\n"),
    ("regtri.pld", "Error in line 8: GAL16V8/20V8: tri. control for reg. output is not allowed\n"),
    ("repar.pld", "Error in line 25: AR is defined twice\n"),
    ("reparst.pld", "Error in line 26: multiple .APRST definitions for the same output\n"),
    ("repclk.pld", "Error in line 9: multiple .CLK definitions for the same output\n"),
    ("repena.pld", "Error in line 19: multiple .E definitions for the same output\n"),
    ("reppin.pld", "Error in line 17: output O4 is defined multiple times\n"),
    ("reprst.pld", "Error in line 26: multiple .ARST definitions for the same output\n"),
    ("repsp.pld", "Error in line 25: SP is defined twice\n"),
    ("spbad.pld", "Error in line 5: GAL22V10: SP is not allowed as pinname\n"),
    ("threeline.pld", "Error in line 2: expected pin definitions, found end of file\n"),
    ("toofewpins.pld", "Error in line 5: wrong number of pins on pin definition line - expected 10, found 9\n"),
    ("toomanyterms_io.pld", "Error in line 7: too many product terms in sum for pin (max: 7, saw: 8)\n"),
    ("twoline.pld", "Error in line 2: expected pin definitions, found end of file\n"),
    ("unkext.pld", "Error in line 7: unknown suffix found: 'UNK'\n"),
    ("unklhs.pld", "Error in line 17: unknown pinname 'DUNNO'\n"),
    ("unkpin.pld", "Error in line 9: unknown pinname 'Unknown'\n"),
    ("unregclk.pld", "Error in line 11: use of .CLK is only allowed for registered outputs\n"),
    ("unregprst.pld", "Error in line 11: use of .APRST is only allowed for registered outputs\n"),
    ("unregrst.pld", "Error in line 11: use of .ARST is only allowed for registered outputs\n"),
];

#[test]
fn test_failing_generation() -> Result<()> {
    let mut failure_messages = HashMap::from(FAILURE_MESSAGES);

    for name in get_plds("testcases/failure")?.iter() {
        let results = get_test_bin("galette")
            .current_dir("testcases/failure")
            .arg(name)
            .output()?;
        check_invocation_failed(name, &failure_messages, results);
        failure_messages.remove(name.as_str());
    }

    assert!(
        failure_messages.is_empty(),
        "Unexercised tests: {:?}",
        failure_messages.keys().collect::<Vec<_>>()
    );

    Ok(())
}
