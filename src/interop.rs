use blueprint::Blueprint;
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
    let c = match parser::parse(file_name) {
        Ok(c) => c,
        Err(e) => { errors::print_error(e); return 1; }
    };

    let mut blueprint = match Blueprint::from(&c) {
        Ok(b) => b,
        Err(e) => { errors::print_error(e); return 1; }
    };

    unsafe { match gal_builder::do_stuff(&mut blueprint, file_name, &(*config)) {
        Ok(()) => 0,
        Err(e) => { errors::print_error(e); 1 }
    } }
}