use chips::Chip;
use jedec::Mode;
use olmc;
use olmc::OLMC;

use std::ffi::CStr;
use std::os::raw::c_char;

const MODE1: i32 =           1;               /* modes (SYN, AC0) */
const MODE2: i32 =           2;
const MODE3: i32 =           3;

#[no_mangle]
pub extern "C" fn new_jedec(gal_type: i32) -> *mut ::jedec::Jedec {
    let gal_type = i32_to_chip(gal_type);
    Box::into_raw(Box::new(::jedec::Jedec::new(gal_type)))
}

#[no_mangle]
pub extern "C" fn clear_row_c(jedec: *mut ::jedec::Jedec, start_row: i32, row_offset: i32) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.clear_row(start_row as usize, row_offset as usize);
}

#[no_mangle]
pub extern "C" fn clear_rows_c(jedec: *mut ::jedec::Jedec, start_row: i32, row_offset: i32, max_row: i32) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.clear_rows(start_row as usize, row_offset as usize, max_row as usize);
}

#[no_mangle]
pub extern "C" fn clear_olmc_c(jedec: *mut ::jedec::Jedec, olmc: i32) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.clear_olmc(olmc as usize);
}

#[no_mangle]
pub extern "C" fn set_ac1(jedec: *mut ::jedec::Jedec, i: usize, ac0: i32) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.ac1[i] = ac0 != 0;
}

#[no_mangle]
pub extern "C" fn set_s1(jedec: *mut ::jedec::Jedec, i: usize, s1: i32) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.s1[i] = s1 != 0;
}

#[no_mangle]
pub extern "C" fn set_pt(jedec: *mut ::jedec::Jedec, i: usize, pt: i32) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.pt[i] = pt != 0;
}

#[no_mangle]
pub extern "C" fn set_xor(jedec: *mut ::jedec::Jedec, i: usize, x: i32) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.xor[i] = x != 0;
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

// Check the OLMC assignments, set AC0 and SYN, and return the mode.
#[no_mangle]
pub extern "C" fn analyse_mode_v8_c(
    jedec: *mut ::jedec::Jedec,
    olmcs: *const OLMC,
) -> i32 {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.check_magic();
    let olmcs = unsafe { std::slice::from_raw_parts(olmcs, 8) };
    match olmc::analyse_mode_v8(jedec, olmcs) {
        Mode::Mode1 => MODE1,
        Mode::Mode2 => MODE2,
        Mode::Mode3 => MODE3,
    }
}

#[no_mangle]
pub extern "C" fn start_row_for_olmc_c(
    gal_type: i32,
    olmc: i32,
) -> i32 {
    let chip = i32_to_chip(gal_type);
    chip.start_row_for_olmc(olmc as usize) as i32
}

#[no_mangle]
pub extern "C" fn num_rows_for_olmc_c(
    gal_type: i32,
    olmc: i32,
) -> i32 {
    let chip = i32_to_chip(gal_type);
    chip.num_rows_for_olmc(olmc as usize) as i32
}
