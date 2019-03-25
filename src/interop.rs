use std::ffi::CStr;
use std::os::raw::c_char;

// IDs used in C.
pub const GAL16V8: i32 = 1;
pub const GAL20V8: i32 = 2;
pub const GAL22V10: i32 = 3;
pub const GAL20RA10: i32 = 4;

pub const MODE1: i32 = 1;
pub const MODE2: i32 = 2;
pub const MODE3: i32 = 3;

// Size of various other fields.
const SIG_SIZE: usize = 64;
const AC1_SIZE: usize = 8;
const PT_SIZE: usize = 64;

// Number of fuses per-row.
pub const ROW_LEN_ADR16: usize = 32;
pub const ROW_LEN_ADR20: usize = 40;
pub const ROW_LEN_ADR22V10: usize = 44;
pub const ROW_LEN_ADR20RA10: usize = 40;

// Number of rows of fuses.
const ROW_COUNT_16V8: usize = 64;
const ROW_COUNT_20V8: usize = 64;
const ROW_COUNT_22V10: usize = 132;
const ROW_COUNT_20RA10: usize = 80;

#[no_mangle]
pub extern "C" fn new_jedec() -> *mut ::jedec::Jedec {
    unsafe { Box::into_raw(Box::new(::jedec::Jedec::new())) }
}

#[no_mangle]
pub extern "C" fn set_syn(jedec: *mut ::jedec::Jedec, syn: i32) {
    let mut jedec: &mut ::jedec::Jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.syn = syn != 0;
}

#[no_mangle]
pub extern "C" fn set_ac0(jedec: *mut ::jedec::Jedec, ac0: i32) {
    let mut jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.ac0 = ac0 != 0;
}

#[no_mangle]
pub extern "C" fn set_ac1(jedec: *mut ::jedec::Jedec, i: usize, ac0: i32) {
    let mut jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.ac1[i] = ac0 != 0;
}

#[no_mangle]
pub extern "C" fn set_s1(jedec: *mut ::jedec::Jedec, i: usize, s1: i32) {
    let mut jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.s1[i] = s1 != 0;
}

#[no_mangle]
pub extern "C" fn set_pt(jedec: *mut ::jedec::Jedec, i: usize, pt: i32) {
    let mut jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.pt[i] = pt != 0;
}

#[no_mangle]
pub extern "C" fn set_xor(jedec: *mut ::jedec::Jedec, i: usize, x: i32) {
    let mut jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.xor[i] = x != 0;
}

#[no_mangle]
pub extern "C" fn set_sig(jedec: *mut ::jedec::Jedec, s: *const c_char) {
    let mut jedec = unsafe { jedec.as_mut().unwrap() };

    let s = unsafe { CStr::from_ptr(s) }.to_bytes();

    // Clear array.
    jedec.sig.iter_mut().map(|x| *x = false);

    // Signature has space for 8 bytes.
    for i in 0..usize::min(s.len(), 8) {
        let c = s[i] as u8;
        for j in 0..8 {
            jedec.sig[i * 8 + j] = (c << j) & 0x80 != 0;
        }
    }
}


#[no_mangle]
pub extern "C" fn write_files_c(
    file_name: *const c_char,
    config: *const ::jedec_writer::Config,
    gal_type: i32,
    mode: i32,
    pin_names: *const * const c_char,
    olmc_pin_types: *const i32,
    gal_fuses: *const u8,
    jedec: *const ::jedec::Jedec
) {
    let jedec = unsafe { jedec.as_ref().unwrap() };
    jedec.check_magic();

    let xor_size = match gal_type {
        GAL16V8 => 8,
        GAL20V8 => 8,
        GAL22V10 => 10,
        GAL20RA10 => 10,
        _ => panic!("Nope"),
    };

    let fuse_size = match gal_type {
        GAL16V8 => ROW_LEN_ADR16 * ROW_COUNT_16V8,
        GAL20V8 => ROW_LEN_ADR20 * ROW_COUNT_20V8,
        GAL22V10 => ROW_LEN_ADR22V10 * ROW_COUNT_22V10,
        GAL20RA10 => ROW_LEN_ADR20RA10 * ROW_COUNT_20RA10,
        _ => panic!("Nope"),
    };

    unsafe {
        let file_name = CStr::from_ptr(file_name);

        let num_pins = if gal_type == GAL16V8 { 20 } else { 24 };
        let cstrs = std::slice::from_raw_parts(pin_names, num_pins);
        let pin_names = cstrs.iter().map(|x| CStr::from_ptr(*x).to_str().unwrap()).collect::<Vec<_>>();

        ::writer::write_files(
            file_name.to_str().unwrap(),
            &(*config),
            gal_type,
            mode,
            &pin_names,
            std::slice::from_raw_parts(olmc_pin_types, 12),
            std::slice::from_raw_parts(gal_fuses, fuse_size),
            &jedec.xor[0..xor_size],
            &jedec.s1[0..10],
            &jedec.sig,
            &jedec.ac1[0..AC1_SIZE],
            &jedec.pt[0..PT_SIZE],
            jedec.syn,
            jedec.ac0,
        );
    }
}
