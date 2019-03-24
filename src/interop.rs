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
pub extern "C" fn write_files_c(
    file_name: *const c_char,
    config: *const ::jedec_writer::Config,
    gal_type: i32,
    mode: i32,
    pin_names: *const * const c_char,
    olmc_pin_types: *const i32,
    gal_fuses: *const u8,
    gal_xor: *const u8,
    gal_s1: *const u8,
    gal_sig: *const u8,
    gal_ac1: *const u8,
    gal_pt: *const u8,
    gal_syn: u8,
    gal_ac0: u8
) {
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
            std::slice::from_raw_parts(gal_xor, xor_size),
            std::slice::from_raw_parts(gal_s1, 10),
            std::slice::from_raw_parts(gal_sig, SIG_SIZE),
            std::slice::from_raw_parts(gal_ac1, AC1_SIZE),
            std::slice::from_raw_parts(gal_pt, PT_SIZE),
            gal_syn,
            gal_ac0,
        );
    }
}
