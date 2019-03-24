use std::ffi::CStr;
use std::fs::File;
use std::os::raw::c_char;
use std::io::Write;

// IDs used in C.
const GAL16V8: i32 = 1;
const GAL20V8: i32 = 2;
const GAL22V10: i32 = 3;
const GAL20RA10: i32 = 4;

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
