pub struct Jedec {
    pub magic: i32,
    pub fuses: Vec<bool>,
    pub xor: Vec<bool>,
    pub sig: Vec<bool>,
    pub ac1: Vec<bool>,
    pub pt: Vec<bool>,
    pub syn: bool,
    pub ac0: bool,
    pub s1: Vec<bool>,
}

// IDs used in C.
pub const GAL16V8: i32 = 1;
pub const GAL20V8: i32 = 2;
pub const GAL22V10: i32 = 3;
pub const GAL20RA10: i32 = 4;

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

// This structure is passed across the C boundary, so let's be careful.
const MAGIC: i32 = 0x12345678;

impl Jedec {
    pub fn new(gal_type: i32) -> Jedec {
        let fuse_size = match gal_type {
            GAL16V8 => ROW_LEN_ADR16 * ROW_COUNT_16V8,
            GAL20V8 => ROW_LEN_ADR20 * ROW_COUNT_20V8,
            GAL22V10 => ROW_LEN_ADR22V10 * ROW_COUNT_22V10,
            GAL20RA10 => ROW_LEN_ADR20RA10 * ROW_COUNT_20RA10,
            _ => panic!("Nope"),
        };

        let xor_size = match gal_type {
            GAL16V8 => 8,
            GAL20V8 => 8,
            GAL22V10 => 10,
            GAL20RA10 => 10,
            _ => panic!("Nope"),
        };

        Jedec {
            magic: MAGIC,
            fuses: vec![true; fuse_size],
            xor: vec![false; xor_size],
            sig: vec![false; 64],
            ac1: vec![false; 8],
            pt: vec![false; 64],
            syn: false,
            ac0: false,
            s1: vec![false; 10],
        }
    }

    pub fn check_magic(&self) {
        assert!(self.magic == MAGIC);
    }
}
