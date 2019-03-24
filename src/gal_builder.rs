// IDs used in C.
const GAL16V8: i32 = 1;
const GAL20V8: i32 = 2;
const GAL22V10: i32 = 3;
const GAL20RA10: i32 = 4;

// Number of fuses per-row.
const ROW_LEN_ADR16: usize = 32;
const ROW_LEN_ADR20: usize = 40;
const ROW_LEN_ADR22V10: usize = 44;
const ROW_LEN_ADR20RA10: usize = 40;

// Number of rows of fuses.
const ROW_COUNT_16V8: usize = 64;
const ROW_COUNT_20V8: usize = 64;
const ROW_COUNT_22V10: usize = 132;
const ROW_COUNT_20RA10: usize = 80;

// GAL16V8

const PinToFuse16Mode1: [i32; 20] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, -1, 30, 26, 22, 18, -1, -1, 14, 10, 6, -1,
];

const PinToFuse16Mode2: [i32; 20] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, -1, 30, -1, 26, 22, 18, 14, 10, 6, -1, -1,
];

const PinToFuse16Mode3: [i32; 20] = [
    -1, 0, 4, 8, 12, 16, 20, 24, 28, -1, -1, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

// GAL20V8

const PinToFuse20Mode1: [i32; 24] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, 38, 34, 30, 26, 22, -1, -1, 18, 14, 10, 6, -1,
];

const PinToFuse20Mode2: [i32; 24] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, 38, 34, -1, 30, 26, 22, 18, 14, 10, -1, 6, -1,
];

const PinToFuse20Mode3: [i32; 24] = [
    -1, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, -1, 38, 34, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

// GAL22V10

const PinToFuse22V10: [i32; 24] = [
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, -1, 42, 38, 34, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

// GAL20RA10

const PinToFuse20RA10: [i32; 24] = [
    -1, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, -1, 38, 34, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

const MODE1: i32 = 1;
const MODE2: i32 = 2;
const MODE3: i32 = 3;

#[no_mangle]
pub extern "C" fn set_and_c(
    fuses: *mut i8,
    gal_s1: *const i8,
    row: u32,
    pin_num: u32,
    negation: u32,
    gal_type: i32,
    mode: i32) {

    let fuse_size = match gal_type {
        GAL16V8 => ROW_LEN_ADR16 * ROW_COUNT_16V8,
        GAL20V8 => ROW_LEN_ADR20 * ROW_COUNT_20V8,
        GAL22V10 => ROW_LEN_ADR22V10 * ROW_COUNT_22V10,
        GAL20RA10 => ROW_LEN_ADR20RA10 * ROW_COUNT_20RA10,
        _ => panic!("Nope"),
    };

    unsafe {
        set_and(std::slice::from_raw_parts_mut(fuses, fuse_size),
                std::slice::from_raw_parts(gal_s1, 10),
                row as usize,
                pin_num as usize,
                negation != 0,
                gal_type,
                mode);
    }
}

// Add an 'and' term to a fuse map.
fn set_and(
    fuses: &mut [i8],
    gal_s1: &[i8],
    row: usize,
    pin_num: usize,
    negation: bool,
    gal_type: i32,
    mode: i32,
) {
    let row_len = match gal_type {
        GAL16V8 => ROW_LEN_ADR16,
        GAL20V8 => ROW_LEN_ADR20,
        GAL22V10 => ROW_LEN_ADR22V10,
        GAL20RA10 => ROW_LEN_ADR20RA10,
        _ => panic!("Nope"),
    };

    let column = match gal_type {
        GAL16V8 => match mode {
            MODE1 => PinToFuse16Mode1[pin_num - 1],
            MODE2 => PinToFuse16Mode2[pin_num - 1],
            MODE3 => PinToFuse16Mode3[pin_num - 1],
            _ => panic!("Nope"),
        },
        GAL20V8 => match mode {
            MODE1 => PinToFuse20Mode1[pin_num - 1],
            MODE2 => PinToFuse20Mode2[pin_num - 1],
            MODE3 => PinToFuse20Mode3[pin_num - 1],
            _ => panic!("Nope"),
        },
        GAL22V10 => PinToFuse22V10[pin_num - 1],
        GAL20RA10 => PinToFuse20RA10[pin_num - 1],
        _ => panic!("Nope"),
    } as usize;

    // Is it a registered OLMC pin?
    // If yes, then correct the negation.
    let mut neg_off = if negation { 1 } else { 0 };
    if gal_type == GAL22V10 && (pin_num >= 14 && pin_num <= 23) && gal_s1[23 - pin_num] == 0 {
        neg_off = 1 - neg_off;
    }

    fuses[row * row_len + column + neg_off] = 0;
}
