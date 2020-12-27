//
// writer.rs: Output functions
//
// This module writes out information about the constructed GAL,
// including the assembled JEDEC file.
//

use itertools::Itertools;
use std::{
    fs::File,
    io::{Error, Write},
    path::PathBuf,
};

use crate::{
    blueprint::OLMC,
    chips::Chip,
    gal::{Mode, GAL},
};

#[derive(Debug)]
pub struct Config {
    pub gen_fuse: bool,
    pub gen_chip: bool,
    pub gen_pin: bool,
    pub jedec_sec_bit: bool,
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
    config: &Config,
    pin_names: &[String],
    olmcs: &[OLMC],
    gal: &GAL,
) -> Result<(), Error> {
    let base = PathBuf::from(file_name);

    write_file(&base, "jed", &make_jedec(config, gal))?;

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

////////////////////////////////////////////////////////////////////////
// 'make_jedec' writes out the assembled JEDEC data.
//

// Structure to track the JEDEC fuse checksum.
struct CheckSummer {
    bit_num: u8,
    byte: u8,
    sum: u16,
}

impl CheckSummer {
    fn new() -> Self {
        CheckSummer {
            bit_num: 0,
            byte: 0,
            sum: 0,
        }
    }

    fn add(&mut self, bit: bool) {
        if bit {
            self.byte |= 1 << self.bit_num
        };
        self.bit_num += 1;
        if self.bit_num == 8 {
            // TODO: Should be mod 0xffff, according to the standard?
            self.sum = self.sum.wrapping_add(self.byte as u16);
            self.byte = 0;
            self.bit_num = 0;
        }
    }

    fn get(&self) -> u16 {
        self.sum + self.byte as u16
    }
}

// A helper to write JEDEC fuse entries into the buffer for the given
// bits, updating the offset and the checksum as we go.
struct FuseBuilder<'a> {
    buf: &'a mut String,
    checksum: CheckSummer,
    idx: usize,
}

impl<'a> FuseBuilder<'a> {
    fn new(buf: &mut String) -> FuseBuilder {
        FuseBuilder {
            buf,
            checksum: CheckSummer::new(),
            idx: 0,
        }
    }

    fn add(&mut self, data: &[bool]) {
        self.add_iter(data.iter());
    }

    fn add_iter<'b, I>(&mut self, data: I)
    where
        I: Iterator<Item = &'b bool>,
    {
        self.buf.push_str(&format!("*L{:04} ", self.idx));
        for bit in data {
            self.buf.push_str(if *bit { "1" } else { "0" });
            self.checksum.add(*bit);
            self.idx += 1;
        }
        self.buf.push('\n');
    }

    // Skip over zeros, updating count and checksum.
    fn skip_iter<'b, I>(&mut self, data: I)
    where
        I: Iterator<Item = &'b bool>,
    {
        for bit in data {
            self.checksum.add(*bit); // (It's a zero.)
            self.idx += 1;
        }
    }

    fn checksum(&mut self) {
        self.buf
            .push_str(&format!("*C{:04x}\n", self.checksum.get()));
    }
}

// Core function to generate a string of the JEDEC file, given the
// config, fuses, etc.
//
// It's galasm-compatible.
pub fn make_jedec(config: &Config, gal: &GAL) -> String {
    let chip = gal.chip;
    let row_len = chip.num_cols();

    let mut buf = String::new();

    buf.push_str("\x02\n");

    buf.push_str(&format!(
        "GAL-Assembler:  Galette {}\n",
        env!("CARGO_PKG_VERSION")
    ));
    buf.push_str(&format!("Device:         {}\n\n", chip.name()));
    // Default value of gal_fuses
    buf.push_str("*F0\n");

    // Security bit state.
    buf.push_str(if config.jedec_sec_bit {
        "*G1\n"
    } else {
        "*G0\n"
    });

    // Number of fuses.
    buf.push_str(&format!("*QF{}\n", chip.total_size()));

    {
        // Construct fuse matrix.
        let mut fuse_builder = FuseBuilder::new(&mut buf);

        // Break the fuse map into chunks representing rows.
        for row in &gal.fuses.iter().chunks(row_len) {
            let (mut check_iter, print_iter) = row.tee();

            // Only write out non-zero bits.
            if check_iter.any(|x| *x) {
                fuse_builder.add_iter(print_iter);
            } else {
                // Process the bits without writing.
                fuse_builder.skip_iter(print_iter);
            }
        }

        // XOR bits are interleaved with S1 bits on GAL22V10 (stored
        // in the 'ac1' field, as it's the same function).
        if chip != Chip::GAL22V10 {
            fuse_builder.add(&gal.xor)
        } else {
            let bits = itertools::interleave(gal.xor.iter(), gal.ac1.iter());
            fuse_builder.add_iter(bits);
        }

        fuse_builder.add(&gal.sig);

        if (chip == Chip::GAL16V8) || (chip == Chip::GAL20V8) {
            fuse_builder.add(&gal.ac1);
            fuse_builder.add(&gal.pt);
            fuse_builder.add(&[gal.syn]);
            fuse_builder.add(&[gal.ac0]);
        }

        // Fuse checksum.
        fuse_builder.checksum();
    }

    buf.push_str("*\n");
    buf.push('\x03');

    // File checksum.
    buf.push_str(&format!("{:04x}\n", file_checksum(buf.as_bytes())));

    buf
}

fn file_checksum(data: &[u8]) -> u16 {
    data.iter().fold(0, |checksum: u16, byte| {
        checksum.wrapping_add(u16::from(*byte))
    })
}

////////////////////////////////////////////////////////////////////////
// 'make_chip' draws out the chip with pin assignments.
//

fn make_chip(chip: Chip, pin_names: &[String]) -> String {
    let num_of_pins = pin_names.len();
    let mut buf = String::new();

    buf.push_str(format!("\n\n{:^72}", chip.name()).trim_end());
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

    buf
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
        buf.push_str(&format!(
            "  {:>2}   | {:<8} | {}\n",
            i,
            name,
            pin_type(gal, olmcs, i)
        ));
    }
    buf.push('\n');

    buf
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

        buf.push(if data[*row * num_of_col + col] {
            '-'
        } else {
            'x'
        });
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
        buf.push_str(&format!(
            "\n\nPin {:>2} = {:<12} {}",
            pin,
            pin_names[pin - 1],
            &flags
        ));

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
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_checksum_wraps() {
        let input = &[0xFF; 0x101];
        assert_eq!(file_checksum(input), 0xFFFF);

        let input = &[0xFF; 0x102];
        assert_eq!(file_checksum(input), 0x00FE);
    }
}
