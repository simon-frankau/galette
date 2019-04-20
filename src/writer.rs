use blueprint::OLMC;
use chips::Chip;
use gal::GAL;
use gal::Mode;
use std::fs::File;
use std::io::Error;
use std::io::Write;
use std::path::PathBuf;

////////////////////////////////////////////////////////////////////////
// 'make_chip' draws out the chip with pin assignments.
//

fn make_chip(gal_type: Chip, pin_names: &[String]) -> String {
    let num_of_pins = pin_names.len();
    let mut buf = String::new();

    buf.push_str(format!("\n\n{:^72}", gal_type.name()).trim_right());
    buf.push_str(&format!("\n\n{:25} -------\\___/-------", ""));

    let mut started = false;
    for n in 0..num_of_pins / 2 {
        if started {
            buf.push_str(&format!("\n{:25} |                 |", ""));
        } else {
            started = true;
        }

        buf.push_str(&format!(
            "\n{:>25} | {:>2}           {:>2} | {}",
            pin_names[n],
            n + 1,
            num_of_pins - n,
            pin_names[num_of_pins - n - 1]
        ));
    }

    buf.push_str(&format!("\n{:25} -------------------\n", ""));

    return buf;
}

////////////////////////////////////////////////////////////////////////
// 'make_pin' lists the pin assignments.
//

fn pin_type(gal: &GAL, olmcs: &[OLMC], i: usize) -> &'static str {
    let chip = gal.chip;
    let num_pins = chip.num_pins();

    if let Some(olmc) = chip.pin_to_olmc(i) {
        let olmc = &olmcs[olmc];
        if olmc.output.is_some() {
            "Output"
        } else if !olmc.feedback {
            "NC"
        } else {
            "Input"
        }
    } else if i == num_pins / 2 {
        "GND"
    } else if i == num_pins {
        "VCC"
    } else {
        match chip {
            Chip::GAL16V8 | Chip::GAL20V8 if gal.get_mode() == Mode::Mode3 && i == 1 => "Clock",
            Chip::GAL16V8 if gal.get_mode() == Mode::Mode3 && i == 11 => "/OE",
            Chip::GAL20V8 if gal.get_mode() == Mode::Mode3 && i == 13 => "/OE",
            Chip::GAL22V10 if i == 1 => "Clock/Input",
            _ => "Input",
        }
    }
}

fn make_pin(gal: &GAL, pin_names: &[String], olmcs: &[OLMC]) -> String {
    let mut buf = String::new();
    buf.push_str("\n\n");
    buf.push_str(" Pin # | Name     | Pin Type\n");
    buf.push_str("-----------------------------\n");

    for (name, i) in pin_names.iter().zip(1..) {
        buf.push_str(&format!("  {:>2}   | {:<8} | {}\n", i, name, pin_type(gal, olmcs, i)));
    }
    buf.push_str("\n");

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
    pin_names: &[String],
    gal: &GAL,
) -> String {
    let mut buf = String::new();

    let chip = gal.chip;
    let num_olmcs = chip.num_olmcs();
    let row_len = chip.num_cols();

    let mut pin = chip.last_olmc();
    let mut row = 0;

    for olmc in 0..num_olmcs {
        if chip == Chip::GAL22V10 && olmc == 0 {
            // AR when 22V10
            buf.push_str("\n\nAR");
            make_row(&mut buf, row_len, row, &gal.fuses);
            row += 1;
        }

        let num_rows = chip.num_rows_for_olmc(olmc);

        // Print pin
        buf.push_str(&format!("\n\nPin {:>2} = ", pin));

        buf.push_str(&format!("{:<13}", pin_names[pin - 1]));

        match chip {
            Chip::GAL16V8 => {
                buf.push_str(&format!("XOR = {:>1}   AC1 = {:>1}", b(gal.xor[19 - pin]), b(gal.ac1[19 - pin])));
            }
            Chip::GAL20V8 => {
                buf.push_str(&format!("XOR = {:>1}   AC1 = {:>1}", b(gal.xor[22 - pin]), b(gal.ac1[22 - pin])));
            }
            Chip::GAL22V10 => {
                buf.push_str(&format!("S0 = {:>1}   S1 = {:>1}", b(gal.xor[23 - pin]), b(gal.s1[23 - pin])));
            }
            Chip::GAL20RA10 => {
                buf.push_str(&format!("S0 = {:>1}", b(gal.xor[23 - pin])));
            }
        };

        for _n in 0..num_rows {
            // Print all fuses of an OLMC
            make_row(&mut buf, row_len, row, &gal.fuses);
            row += 1;
        }

        if chip == Chip::GAL22V10 && olmc == 9 {
            // SP when 22V10
            buf.push_str("\n\nSP");
            make_row(&mut buf, row_len, row, &gal.fuses);
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
    pin_names: &[String],
    olmcs: &[OLMC],
    gal: &GAL,
) -> Result<(), Error> {
    let base = PathBuf::from(file_name);

    write_file(&base, "jed", &::jedec_writer::make_jedec(config, gal))?;

    if config.gen_fuse {
        write_file(&base, "fus", &make_fuse(pin_names, gal))?;
    }

    if config.gen_pin {
        write_file(&base, "pin", &make_pin(gal, pin_names, olmcs))?;
    }

    if config.gen_chip {
        write_file(&base, "chp", &make_chip(gal.chip, pin_names))?;
    }

    Ok(())
}
