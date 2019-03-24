use std::ffi::CStr;
use std::fs::File;
use std::os::raw::c_char;
use std::io::Write;

// IDs used in C.
const GAL16V8: i32 = 1;
const GAL20V8: i32 = 2;
const GAL22V10: i32 = 3;
const GAL20RA10: i32 = 4;

const MODE1: i32 = 1;
const MODE2: i32 = 2;
const MODE3: i32 = 3;

const INPUT: i32 = 2;

#[no_mangle]
pub extern "C" fn write_chip_c(
    file_name: *const c_char,
    gal_type: i32,
    pin_names: *const * const c_char,
) {
    unsafe {
        let file_name_rs = CStr::from_ptr(file_name);

        let num_pins = if gal_type == GAL16V8 { 20 } else { 24 };

        let cstrs = std::slice::from_raw_parts(pin_names, num_pins);
        let pin_names = cstrs.iter().map(|x| CStr::from_ptr(*x).to_str().unwrap()).collect::<Vec<_>>();

        let str = make_chip(gal_type, &pin_names);

        let mut file = File::create(file_name_rs.to_str().unwrap()).unwrap();
        file.write_all(str.as_bytes());
    }
}

fn make_spaces(buf: &mut String, n: usize) {
    for _i in 0..n {
        buf.push(' ');
    }
}

fn make_chip(gal_type: i32, pin_names: &[&str]) -> String {
    let num_of_pins = pin_names.len();
    let mut buf = String::new();

    buf.push_str("\n\n");

    make_spaces(&mut buf, 31);

    buf.push_str(match gal_type {
        GAL16V8   => " GAL16V8\n\n",
        GAL20V8   => " GAL20V8\n\n",
        GAL22V10  => " GAL22V10\n\n",
        GAL20RA10 => "GAL20RA10\n\n",
        _ => panic!("Nope"),
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

        buf.push_str(&format!("{} | {:>2}           {:>2} | {}\n",
                     pin_names[n],
                     n + 1,
                     num_of_pins - n,
                     pin_names[num_of_pins - n - 1]));
    }

    make_spaces(&mut buf, 26);
    buf.push_str("-------------------\n");

    return buf;
}

const DUMMY_OLMC12: usize = 25;

fn is_olmc(gal_type: i32, n: usize) -> bool {
    match gal_type {
    GAL16V8 => n >= 12 && n <= 19,
    GAL20V8 => n >= 15 && n <= 22,
    GAL22V10 => n >= 14 && n <= DUMMY_OLMC12,
    GAL20RA10 => n >= 14 && n <= 23,
    _ => panic!("Nope"),
    }
}

fn pin_to_olmc(gal_type: i32, pin: usize) -> usize {
    pin - match gal_type {
        GAL16V8 => 12,
        GAL20V8 => 15,
        GAL22V10 => 14,
        GAL20RA10 => 14,
        _ => panic!("Nope")
    }
}

#[no_mangle]
pub extern "C" fn write_pin_c(
    file_name: *const c_char,
    gal_type: i32,
    pin_names: *const * const c_char,
    mode: i32,
    olmc_pin_types: *const i32
) {
    unsafe {
        let file_name_rs = CStr::from_ptr(file_name);

        let num_pins = if gal_type == GAL16V8 { 20 } else { 24 };

        let cstrs = std::slice::from_raw_parts(pin_names, num_pins);
        let pin_names = cstrs.iter().map(|x| CStr::from_ptr(*x).to_str().unwrap()).collect::<Vec<_>>();
        let olmc_pin_types_slice = std::slice::from_raw_parts(olmc_pin_types, 12);

        let str = make_pin(gal_type, &pin_names, mode, olmc_pin_types_slice);

        let mut file = File::create(file_name_rs.to_str().unwrap()).unwrap();
        file.write_all(str.as_bytes());
    }
}



fn make_pin(gal_type: i32, pin_names: &[&str], mode: i32, olmc_pin_types: &[i32]) -> String {
    let num_of_pins = pin_names.len();

    let mut buf = String::new();
    buf.push_str("\n\n");
    buf.push_str(" Pin # | Name     | Pin Type\n");
    buf.push_str("-----------------------------\n");

    for n in 1..num_of_pins + 1 {
        buf.push_str(&format!("  {:>2}   | ", n));
        buf.push_str(pin_names[n - 1]);

        make_spaces(&mut buf, 9 - pin_names[n-1].len());

        let mut flag = false;

        if n == num_of_pins / 2 {
            buf.push_str("| GND\n");
            flag = true;
        }

        if n == num_of_pins {
            buf.push_str("| VCC\n\n");
            flag = true;
        }

        if gal_type == GAL16V8 || gal_type == GAL20V8 {
            if mode == MODE3 && n == 1 {
                buf.push_str("| Clock\n");
                flag = true;
            }

            if mode == MODE3 {
                if gal_type == GAL16V8 && n == 11 {
                    buf.push_str("| /OE\n");
                    flag = true;
                }

                if gal_type == GAL20V8 && n == 13 {
                    buf.push_str("| /OE\n");
                    flag = true;
                }
            }
        }

        if gal_type == GAL22V10 && n == 1 {
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
