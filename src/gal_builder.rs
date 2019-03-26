use chips::Chip;
use interop;

// GAL16V8

const PIN_TO_FUSE_16_MODE1: [i32; 20] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, -1, 30, 26, 22, 18, -1, -1, 14, 10, 6, -1,
];

const PIN_TO_FUSE_16_MODE2: [i32; 20] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, -1, 30, -1, 26, 22, 18, 14, 10, 6, -1, -1,
];

const PIN_TO_FUSE_16_MODE3: [i32; 20] = [
    -1, 0, 4, 8, 12, 16, 20, 24, 28, -1, -1, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

// GAL20V8

const PIN_TO_FUSE_20_MODE1: [i32; 24] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, 38, 34, 30, 26, 22, -1, -1, 18, 14, 10, 6, -1,
];

const PIN_TO_FUSE_20_MODE2: [i32; 24] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, 38, 34, -1, 30, 26, 22, 18, 14, 10, -1, 6, -1,
];

const PIN_TO_FUSE_20_MODE3: [i32; 24] = [
    -1, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, -1, 38, 34, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

// GAL22V10

const PIN_TO_FUSE_22V10: [i32; 24] = [
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, -1, 42, 38, 34, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

// GAL20RA10

const PIN_TO_FUSE_20RA10: [i32; 24] = [
    -1, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, -1, 38, 34, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

const MODE1: i32 = 1;
const MODE2: i32 = 2;
const MODE3: i32 = 3;

#[no_mangle]
pub extern "C" fn set_and_c(
    jedec: *mut ::jedec::Jedec,
    row: u32,
    pin_num: u32,
    negation: u32,
    gal_type: i32,
    mode: i32,
) {
    let jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.check_magic();
    let gal_type = interop::i32_to_chip(gal_type);

    set_and(
        &mut jedec.fuses,
        &jedec.s1,
        row as usize,
        pin_num as usize,
        negation != 0,
        gal_type,
        mode,
    );
}

// Add an 'and' term to a fuse map.
fn set_and(
    fuses: &mut [bool],
    s1: &[bool],
    row: usize,
    pin_num: usize,
    negation: bool,
    gal_type: Chip,
    mode: i32,
) {
    let row_len = gal_type.num_cols();

    let column_lookup: &[i32] = match gal_type {
        Chip::GAL16V8 => match mode {
            MODE1 => &PIN_TO_FUSE_16_MODE1,
            MODE2 => &PIN_TO_FUSE_16_MODE2,
            MODE3 => &PIN_TO_FUSE_16_MODE3,
            _ => panic!("Nope"),
        },
        Chip::GAL20V8 => match mode {
            MODE1 => &PIN_TO_FUSE_20_MODE1,
            MODE2 => &PIN_TO_FUSE_20_MODE2,
            MODE3 => &PIN_TO_FUSE_20_MODE3,
            _ => panic!("Nope"),
        },
        Chip::GAL22V10 => &PIN_TO_FUSE_22V10,
        Chip::GAL20RA10 => &PIN_TO_FUSE_20RA10,
    };

    let column = column_lookup[pin_num - 1] as usize;

    // Is it a registered OLMC pin?
    // If yes, then correct the negation.
    let mut neg_off = if negation { 1 } else { 0 };
    if gal_type == Chip::GAL22V10 && (pin_num >= 14 && pin_num <= 23) && !s1[23 - pin_num] {
        neg_off = 1 - neg_off;
    }

    fuses[row * row_len + column + neg_off] = false;
}
