use chips::Chip;
use gal_builder;
use gal_builder::Equation;
use olmc::OLMC;

use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn new_jedec(gal_type: i32) -> *mut ::jedec::Jedec {
    let gal_type = i32_to_chip(gal_type);
    Box::into_raw(Box::new(::jedec::Jedec::new(gal_type)))
}

#[no_mangle]
pub extern "C" fn set_sig(jedec: *mut ::jedec::Jedec, s: *const c_char) {
    let jedec = unsafe { jedec.as_mut().unwrap() };

    let s = unsafe { CStr::from_ptr(s) }.to_bytes();

    // Clear array.
    for x in jedec.sig.iter_mut() {
        *x = false;
    }

    // Signature has space for 8 bytes.
    for i in 0..usize::min(s.len(), 8) {
        let c = s[i] as u8;
        for j in 0..8 {
            jedec.sig[i * 8 + j] = (c << j) & 0x80 != 0;
        }
    }
}

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
pub extern "C" fn write_files_c(
    file_name: *const c_char,
    config: *const ::jedec_writer::Config,
    pin_names: *const *const c_char,
    olmc_pin_types: *const i32,
    jedec: *const ::jedec::Jedec,
) {
    let jedec = unsafe { jedec.as_ref().unwrap() };
    jedec.check_magic();

    unsafe {
        let file_name = CStr::from_ptr(file_name);

        let num_pins = if jedec.chip == Chip::GAL16V8 { 20 } else { 24 };
        let cstrs = std::slice::from_raw_parts(pin_names, num_pins);
        let pin_names = cstrs
            .iter()
            .map(|x| CStr::from_ptr(*x).to_str().unwrap())
            .collect::<Vec<_>>();

        ::writer::write_files(
            file_name.to_str().unwrap(),
            &(*config),
            &pin_names,
            std::slice::from_raw_parts(olmc_pin_types, 12),
            jedec
        ).unwrap();
    }
}

#[no_mangle]
pub extern "C" fn set_unused_c(jedec: *mut ::jedec::Jedec, olmcs: *const OLMC) -> i32{
    let jedec = unsafe { jedec.as_mut().unwrap() };
    // TODO: This was "jedec.chip.num_olmcs())", but AR and SP are special...
    let olmcs = unsafe { std::slice::from_raw_parts(olmcs, 12) };
    match gal_builder::set_unused(jedec, olmcs) {
        Ok(_) => 0,
        Err(i) => i as i32,
    }
}

#[no_mangle]
pub extern "C" fn do_it_all_c(
    jedec: *mut ::jedec::Jedec,
    olmcs: *mut OLMC,
    eqns: *const Equation,
    num_eqns: i32,
    file_name: *const c_char,
) -> i32 {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.check_magic();
    let olmcs = unsafe { std::slice::from_raw_parts_mut(olmcs, 12) };
    let eqns = unsafe { std::slice::from_raw_parts(eqns, num_eqns as usize) };
    let file_name = unsafe {CStr::from_ptr(file_name) };

    match gal_builder::do_it_all(jedec, olmcs, eqns, file_name.to_str().unwrap()) {
        Ok(()) => 0,
        Err(i) => i,
    }
}

#[no_mangle]
pub extern "C" fn do_stuff_c(
    gal_type: i32,
    sig: *const c_char,
    eqns: *const Equation,
    num_eqns: i32,
    file_name: *const c_char,
    pin_names: *const *const c_char,
    config: *const ::jedec_writer::Config,
) -> i32 {
    let gal_type = i32_to_chip(gal_type);
    let sig = unsafe { CStr::from_ptr(sig) }.to_bytes();
    let eqns = unsafe { std::slice::from_raw_parts(eqns, num_eqns as usize) };
    let file_name = unsafe {CStr::from_ptr(file_name) };

    let num_pins = if gal_type == Chip::GAL16V8 { 20 } else { 24 };
    let cstrs = unsafe { std::slice::from_raw_parts(pin_names, num_pins) };
    let pin_names = cstrs
        .iter()
        .map(|x| unsafe { CStr::from_ptr(*x).to_str().unwrap() })
        .collect::<Vec<_>>();

    unsafe { match gal_builder::do_stuff(gal_type, sig, eqns, file_name.to_str().unwrap(), &pin_names, &(*config)) {
        Ok(()) => 0,
        Err(i) => i,
    } }
}