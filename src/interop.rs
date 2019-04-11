use chips::Chip;
use errors;
use gal_builder;
use parser;

use std::ffi::CStr;
use std::os::raw::c_char;

pub fn i32_to_chip(gal_type: i32) -> Chip {
    match gal_type {
        1 => Chip::GAL16V8,
        2 => Chip::GAL20V8,
        3 => Chip::GAL22V10,
        4 => Chip::GAL20RA10,
        _ => panic!("Nope")
    }
}

#[no_mangle]
pub extern "C" fn do_stuff_c(
    file_name: *const c_char,
    config: *const ::jedec_writer::Config,
) -> i32 {
    let file_name = unsafe {CStr::from_ptr(file_name) };
    let file_name = file_name.to_str().unwrap();

    println!("Assembler Phase 1 for \"{}\"", file_name);
    let c = match parser::parse_stuff(file_name) {
        Ok(c) => c,
        Err(e) => { errors::print_error(e); return 1; }
    };

    let mut pin_names = Vec::new();
    for (name, neg) in c.pins.iter() {
        let mut full_name = if *neg { String::from("/") } else { String::new() };
        full_name.push_str(&name);
        pin_names.push(full_name);
    }
    let pin_names_ref = pin_names.iter().map(String::as_ref).collect::<Vec<&str>>();

    unsafe { match gal_builder::do_stuff(c.chip, &c.sig, &c.eqns, file_name, &pin_names_ref, &(*config)) {
        Ok(()) => 0,
        Err(e) => { errors::print_error(e); 1 }
    } }
}