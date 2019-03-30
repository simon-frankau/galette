use chips::Chip;
use jedec::Jedec;
use std::fs::File;
use std::io::Error;
use std::io::Write;
use std::path::PathBuf;

const INPUT: i32 = 2;

pub const MODE1: i32 = 1;
pub const MODE2: i32 = 2;
pub const MODE3: i32 = 3;

fn make_spaces(buf: &mut String, n: usize) {
    for _i in 0..n {
        buf.push(' ');
    }
}

////////////////////////////////////////////////////////////////////////
// 'make_chip' draws out the chip with pin assignments.
//

fn make_chip(gal_type: Chip, pin_names: &[&str]) -> String {
    let num_of_pins = pin_names.len();
    let mut buf = String::new();

    buf.push_str("\n\n");

    make_spaces(&mut buf, 31);

    // TODO: Shuffle name a space left for the RA10 for alignment.
    buf.push_str(&format!(" {}\n\n", gal_type.name()));

    make_spaces(&mut buf, 26);

    buf.push_str("-------\\___/-------\n");

    let mut started = false;
    for n in 0..num_of_pins / 2 {
        if started {
            make_spaces(&mut buf, 26);
            buf.push_str("|                 |\n");
        } else {
            started = true;
        }

        make_spaces(&mut buf, 25 - pin_names[n].len());

        buf.push_str(&format!(
            "{} | {:>2}           {:>2} | {}\n",
            pin_names[n],
            n + 1,
            num_of_pins - n,
            pin_names[num_of_pins - n - 1]
        ));
    }

    make_spaces(&mut buf, 26);
    buf.push_str("-------------------\n");

    return buf;
}

////////////////////////////////////////////////////////////////////////
// 'make_pin' lists the pin assignments.
//

fn make_pin(gal_type: Chip, pin_names: &[&str], mode: i32, olmc_pin_types: &[i32]) -> String {
    let num_of_pins = pin_names.len();

    let mut buf = String::new();
    buf.push_str("\n\n");
    buf.push_str(" Pin # | Name     | Pin Type\n");
    buf.push_str("-----------------------------\n");

    for n in 1..num_of_pins + 1 {
        buf.push_str(&format!("  {:>2}   | ", n));
        buf.push_str(pin_names[n - 1]);

        make_spaces(&mut buf, 9 - pin_names[n - 1].len());

        let mut flag = false;

        if n == num_of_pins / 2 {
            buf.push_str("| GND\n");
            flag = true;
        }

        if n == num_of_pins {
            buf.push_str("| VCC\n\n");
            flag = true;
        }

        if gal_type == Chip::GAL16V8 || gal_type == Chip::GAL20V8 {
            if mode == MODE3 && n == 1 {
                buf.push_str("| Clock\n");
                flag = true;
            }

            if mode == MODE3 {
                if gal_type == Chip::GAL16V8 && n == 11 {
                    buf.push_str("| /OE\n");
                    flag = true;
                }

                if gal_type == Chip::GAL20V8 && n == 13 {
                    buf.push_str("| /OE\n");
                    flag = true;
                }
            }
        }

        if gal_type == Chip::GAL22V10 && n == 1 {
            buf.push_str("| Clock/Input\n");
            flag = true;
        }

        if let Some(olmc) = gal_type.pin_to_olmc(n) {
            if olmc_pin_types[olmc] != INPUT {
                if olmc_pin_types[olmc] != 0 {
                    buf.push_str("| Output\n");
                } else {
                    buf.push_str("| NC\n");
                }
            } else {
                buf.push_str("| Input\n");
            }
        } else {
            if !flag {
                buf.push_str("| Input\n");
            }
        }
    }

    return buf;
}

////////////////////////////////////////////////////////////////////////
// 'make_fuse' writes out a fuse map.
//

fn make_row(buf: &mut String, num_of_col: usize, row: usize, data: &[bool]) {
    buf.push_str(&format!("\n{:>3} ", row));

    for col in 0..num_of_col {
        if col % 4 == 0 {
            buf.push_str(" ");
        }

        if data[row * num_of_col + col] {
            buf.push_str("-");
        } else {
            buf.push_str("x");
        }
    }
}

// Short-named helper
fn b(bit: bool) -> char {
    if bit {
        '1'
    } else {
        '0'
    }
}

fn make_fuse(
    pin_names: &[&str],
    jedec: &Jedec,
) -> String {
    let mut buf = String::new();

    let gal_type = jedec.chip;
    let num_olmcs = gal_type.num_olmcs();
    let row_len = gal_type.num_cols();

    let mut pin = gal_type.last_olmc();
    let mut row = 0;

    for olmc in 0..num_olmcs {
        if gal_type == Chip::GAL22V10 && olmc == 0 {
            // AR when 22V10
            buf.push_str("\n\nAR");
            make_row(&mut buf, row_len, row, &jedec.fuses);
            row += 1;
        }

        let num_rows = gal_type.num_rows_for_olmc(olmc);

        // Print pin
        buf.push_str(&format!("\n\nPin {:>2} = ", pin));

        buf.push_str(&format!("{}", pin_names[pin - 1]));

        make_spaces(&mut buf, 13 - pin_names[pin - 1].len());

        match gal_type {
            Chip::GAL16V8 => {
                buf.push_str(&format!("XOR = {:>1}   AC1 = {:>1}", b(jedec.xor[19 - pin]), b(jedec.ac1[19 - pin])));
            }
            Chip::GAL20V8 => {
                buf.push_str(&format!("XOR = {:>1}   AC1 = {:>1}", b(jedec.xor[22 - pin]), b(jedec.ac1[22 - pin])));
            }
            Chip::GAL22V10 => {
                buf.push_str(&format!("S0 = {:>1}   S1 = {:>1}", b(jedec.xor[23 - pin]), b(jedec.s1[23 - pin])));
            }
            Chip::GAL20RA10 => {
                buf.push_str(&format!("S0 = {:>1}", b(jedec.xor[23 - pin])));
            }
        };

        for _n in 0..num_rows {
            // Print all fuses of an OLMC
            make_row(&mut buf, row_len, row, &jedec.fuses);
            row += 1;
        }

        if gal_type == Chip::GAL22V10 && olmc == 9 {
            // SP when 22V10
            buf.push_str("\n\nSP");
            make_row(&mut buf, row_len, row, &jedec.fuses);
        }

        pin -= 1;
    }

    buf.push_str("\n\n");
    return buf;
}

////////////////////////////////////////////////////////////////////////
// Main entry point for writing all the files is 'write_files'.
//

fn write_file(base: &PathBuf, ext: &str, buf: &str) -> Result<(), Error> {
    let mut file = File::create(base.with_extension(ext).to_str().unwrap())?;
    file.write_all(buf.as_bytes())?;
    Ok(())
}

pub fn write_files(
    file_name: &str,
    config: &::jedec_writer::Config,
    mode: i32,
    pin_names: &[&str],
    olmc_pin_types: &[i32],
    jedec: &Jedec,
) -> Result<(), Error> {
    let base = PathBuf::from(file_name);
    let gal_type = jedec.chip;

    write_file(&base, "jed", &::jedec_writer::make_jedec(config, jedec))?;

    if config.gen_fuse != 0 {
        write_file(&base, "fus", &make_fuse(pin_names, jedec))?;
    }

    if config.gen_pin != 0 {
        write_file(&base, "pin", &make_pin(gal_type, pin_names, mode, olmc_pin_types))?;
    }

    if config.gen_chip != 0 {
        write_file(&base, "chp", &make_chip(gal_type, pin_names))?;
    }

    Ok(())
}