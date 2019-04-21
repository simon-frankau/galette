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

fn make_row(buf: &mut String, row: &mut usize, num_of_col: usize, data: &[bool]) {
    buf.push_str(&format!("\n{:>3} ", row));

    for col in 0..num_of_col {
        if col % 4 == 0 {
            buf.push(' ');
        }

        buf.push(if data[*row * num_of_col + col] { '-' } else { 'x' });
    }

    *row += 1;
}

fn to_bit(bit: bool) -> char {
    if bit {
        '1'
    } else {
        '0'
    }
}

fn make_fuse(pin_names: &[String], gal: &GAL) -> String {

    // This function relies on detailed knowledge of the ordering of
    // rows in the fuse map vs. OLMCs vs. pins. It's brittle, but
    // no-one's changing the hardware layout. :)

    let mut buf = String::new();

    let chip = gal.chip;
    let row_len = chip.num_cols();

    let mut pin = chip.last_olmc();
    let mut row = 0;

    // AR for the 22V10
    if chip == Chip::GAL22V10 {
        buf.push_str("\n\nAR");
        make_row(&mut buf, &mut row, row_len, &gal.fuses);
    }

    let last_olmc = chip.last_olmc();
    for olmc in 0..chip.num_olmcs() {
        let xor = to_bit(gal.xor[last_olmc - pin]);
        let ac1 = to_bit(gal.ac1[last_olmc - pin]);
        let flags = match chip {
            Chip::GAL16V8 => format!("XOR = {:>1}   AC1 = {:>1}", xor, ac1),
            Chip::GAL20V8 => format!("XOR = {:>1}   AC1 = {:>1}", xor, ac1),
            Chip::GAL22V10 => format!("S0 = {:>1}   S1 = {:>1}", xor, ac1),
            Chip::GAL20RA10 => format!("S0 = {:>1}", xor),
        };
        buf.push_str(&format!("\n\nPin {:>2} = {:<12} {}", pin, pin_names[pin - 1], &flags));

        for _ in 0..chip.num_rows_for_olmc(olmc) {
            // Print all fuses of an OLMC
            make_row(&mut buf, &mut row, row_len, &gal.fuses);
        }

        pin -= 1;
    }

    // SP for the 22V10
    if chip == Chip::GAL22V10 {
        buf.push_str("\n\nSP");
        make_row(&mut buf, &mut row, row_len, &gal.fuses);
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
