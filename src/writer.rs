use chips::Chip;
use std::fs::File;
use std::io::Error;
use std::io::Write;
use std::path::PathBuf;

const INPUT: i32 = 2;

// Number of fuses per-row.
pub const ROW_LEN_ADR16: usize = 32;
pub const ROW_LEN_ADR20: usize = 40;
pub const ROW_LEN_ADR22V10: usize = 44;
pub const ROW_LEN_ADR20RA10: usize = 40;

pub const MODE1: i32 = 1;
pub const MODE2: i32 = 2;
pub const MODE3: i32 = 3;

fn make_spaces(buf: &mut String, n: usize) {
    for _i in 0..n {
        buf.push(' ');
    }
}

fn make_chip(gal_type: Chip, pin_names: &[&str]) -> String {
    let num_of_pins = pin_names.len();
    let mut buf = String::new();

    buf.push_str("\n\n");

    make_spaces(&mut buf, 31);

    buf.push_str(match gal_type {
        Chip::GAL16V8 => " GAL16V8\n\n",
        Chip::GAL20V8 => " GAL20V8\n\n",
        Chip::GAL22V10 => " GAL22V10\n\n",
        Chip::GAL20RA10 => "GAL20RA10\n\n",
    });

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

const DUMMY_OLMC12: usize = 25;

fn is_olmc(gal_type: Chip, n: usize) -> bool {
    match gal_type {
        Chip::GAL16V8 => n >= 12 && n <= 19,
        Chip::GAL20V8 => n >= 15 && n <= 22,
        Chip::GAL22V10 => n >= 14 && n <= DUMMY_OLMC12,
        Chip::GAL20RA10 => n >= 14 && n <= 23,
    }
}

fn pin_to_olmc(gal_type: Chip, pin: usize) -> usize {
    pin - match gal_type {
        Chip::GAL16V8 => 12,
        Chip::GAL20V8 => 15,
        Chip::GAL22V10 => 14,
        Chip::GAL20RA10 => 14,
    }
}

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

        // OLMC pin?
        // Second condition is a hack as VCC is a dummy OLMC on a 22V10.
        if is_olmc(gal_type, n) && n < 24 {
            let k = pin_to_olmc(gal_type, n);
            if olmc_pin_types[k] != INPUT {
                if olmc_pin_types[k] != 0 {
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

const OLMC_SIZE_22V10: [i32; 12] = [9, 11, 13, 15, 17, 17, 15, 13, 11, 9, 1, 1];

fn get_size(gal_type: Chip, olmc: usize) -> i32 {
    match gal_type {
        Chip::GAL16V8 => 8,
        Chip::GAL20V8 => 8,
        Chip::GAL22V10 => OLMC_SIZE_22V10[olmc],
        Chip::GAL20RA10 => 8,
    }
}

fn make_fuse(
    gal_type: Chip,
    pin_names: &[&str],
    gal_fuse: &[bool],
    gal_xor: &[bool],
    gal_ac1: &[bool],
    gal_s1: &[bool],
) -> String {
    let mut buf = String::new();

    let (mut pin, num_olmcs) = match gal_type {
        Chip::GAL16V8 => (19, 8),
        Chip::GAL20V8 => (22, 8),
        Chip::GAL22V10 => (23, 10),
        Chip::GAL20RA10 => (23, 10),
    };

    let row_len = match gal_type {
        Chip::GAL16V8 => ROW_LEN_ADR16,
        Chip::GAL20V8 => ROW_LEN_ADR20,
        Chip::GAL22V10 => ROW_LEN_ADR22V10,
        Chip::GAL20RA10 => ROW_LEN_ADR20RA10,
    };

    let mut row = 0;

    for olmc in 0..num_olmcs {
        if gal_type == Chip::GAL22V10 && olmc == 0 {
            // AR when 22V10
            buf.push_str("\n\nAR");
            make_row(&mut buf, row_len, row, gal_fuse);
            row += 1;
        }

        let num_rows = get_size(gal_type, olmc);

        // Print pin
        buf.push_str(&format!("\n\nPin {:>2} = ", pin));

        buf.push_str(&format!("{}", pin_names[pin - 1]));

        make_spaces(&mut buf, 13 - pin_names[pin - 1].len());

        match gal_type {
            Chip::GAL16V8 => {
                buf.push_str(&format!(
                    "XOR = {:>1}   AC1 = {:>1}",
                    if gal_xor[19 - pin] { 1 } else { 0 },
                    if gal_ac1[19 - pin] { 1 } else { 0 }
                ));
            }
            Chip::GAL20V8 => {
                buf.push_str(&format!(
                    "XOR = {:>1}   AC1 = {:>1}",
                    if gal_xor[22 - pin] { 1 } else { 0 },
                    if gal_ac1[22 - pin] { 1 } else { 0 }
                ));
            }
            Chip::GAL22V10 => {
                buf.push_str(&format!(
                    "S0 = {:>1}   S1 = {:>1}",
                    if gal_xor[23 - pin] { 1 } else { 0 },
                    if gal_s1[23 - pin] { 1 } else { 0 }
                ));
            }
            Chip::GAL20RA10 => {
                buf.push_str(&format!(
                    "S0 = {:>1}",
                    if gal_xor[23 - pin] { 1 } else { 0 }
                ));
            }
        };

        for _n in 0..num_rows {
            // Print all fuses of an OLMC
            make_row(&mut buf, row_len, row, gal_fuse);
            row += 1;
        }

        if gal_type == Chip::GAL22V10 && olmc == 9 {
            // SP when 22V10
            buf.push_str("\n\nSP");
            make_row(&mut buf, row_len, row, gal_fuse);
        }

        pin -= 1;
    }

    buf.push_str("\n\n");
    return buf;
}

pub fn write_files(
    file_name: &str,
    config: &::jedec_writer::Config,
    gal_type: Chip,
    mode: i32,
    pin_names: &[&str],
    olmc_pin_types: &[i32],
    gal_fuses: &[bool],
    gal_xor: &[bool],
    gal_s1: &[bool],
    gal_sig: &[bool],
    gal_ac1: &[bool],
    gal_pt: &[bool],
    gal_syn: bool,
    gal_ac0: bool,
) -> Result<(), Error> {
    let base = PathBuf::from(file_name);

    {
        let buf = ::jedec_writer::make_jedec(
            gal_type, config, gal_fuses, gal_xor, gal_s1, gal_sig, gal_ac1, gal_pt, gal_syn,
            gal_ac0,
        );
        let mut file = File::create(base.with_extension("jed").to_str().unwrap())?;
        file.write_all(buf.as_bytes())?;
    }

    if config.gen_fuse != 0 {
        let buf = make_fuse(gal_type, pin_names, gal_fuses, gal_xor, gal_ac1, gal_s1);
        let mut file = File::create(base.with_extension("fus").to_str().unwrap())?;
        file.write_all(buf.as_bytes())?;
    }

    if config.gen_pin != 0 {
        let buf = make_pin(gal_type, pin_names, mode, olmc_pin_types);
        let mut file = File::create(base.with_extension("pin").to_str().unwrap())?;
        file.write_all(buf.as_bytes())?;
    }

    if config.gen_chip != 0 {
        let buf = make_chip(gal_type, pin_names);
        let mut file = File::create(base.with_extension("chp").to_str().unwrap())?;
        file.write_all(buf.as_bytes())?;
    }

    Ok(())
}
