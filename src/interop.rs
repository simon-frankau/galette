use std::ffi::CStr;
use std::os::raw::c_char;

// IDs used in C.
pub const GAL16V8: i32 = 1;
pub const GAL20V8: i32 = 2;
pub const GAL22V10: i32 = 3;
pub const GAL20RA10: i32 = 4;

#[no_mangle]
pub extern "C" fn new_jedec(gal_type: i32) -> *mut ::jedec::Jedec {
    Box::into_raw(Box::new(::jedec::Jedec::new(gal_type)))
}

#[no_mangle]
pub extern "C" fn set_fuse(jedec: *mut ::jedec::Jedec, i: usize, x: i32) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.fuses[i] = x != 0;
}

#[no_mangle]
pub extern "C" fn set_syn(jedec: *mut ::jedec::Jedec, syn: i32) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.syn = syn != 0;
}

#[no_mangle]
pub extern "C" fn set_ac0(jedec: *mut ::jedec::Jedec, ac0: i32) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.ac0 = ac0 != 0;
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


#[no_mangle]
pub extern "C" fn write_files_c(
    file_name: *const c_char,
    config: *const ::jedec_writer::Config,
    gal_type: i32,
    mode: i32,
    pin_names: *const * const c_char,
    olmc_pin_types: *const i32,
    jedec: *const ::jedec::Jedec
) {
    let jedec = unsafe { jedec.as_ref().unwrap() };
    jedec.check_magic();

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
            &jedec.fuses,
            &jedec.xor,
            &jedec.s1,
            &jedec.sig,
            &jedec.ac1,
            &jedec.pt,
            jedec.syn,
            jedec.ac0,
        ).unwrap();
    }
}
