use chips::Chip;

pub struct Jedec {
    pub chip: Chip,
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

// Mode enums for the v8s
#[derive(PartialEq,Clone,Copy)]
pub enum Mode {
    Mode1,
    Mode2,
    Mode3,
}

// This structure is passed across the C boundary, so let's be careful.
const MAGIC: i32 = 0x12345678;

impl Jedec {
    pub fn new(gal_type: Chip) -> Jedec {

        let fuse_size = gal_type.logic_size();
        let num_olmcs = gal_type.num_olmcs();

        Jedec {
            chip: gal_type,
            magic: MAGIC,
            fuses: vec![true; fuse_size],
            // One xor bit per OLMC.
            xor: vec![false; num_olmcs],
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

    pub fn clear_row(&mut self, start_row: usize, row_offset: usize) {
        let num_cols = self.chip.num_cols();
        let start = (start_row + row_offset) * num_cols;
        for i in start .. start + num_cols {
            self.fuses[i] = false;
        }
    }

    pub fn clear_rows(&mut self, start_row: usize, row_offset: usize, max_row: usize) {
        let num_cols = self.chip.num_cols();
        let start = (start_row + row_offset) * num_cols;
        let end = (start_row + max_row) * num_cols;
        for i in start .. end {
            self.fuses[i] = false;
        }
    }

    pub fn clear_olmc(&mut self, olmc: usize) {
        let num_cols = self.chip.num_cols();
        let start = self.chip.start_row_for_olmc(olmc);
        let end = start + self.chip.num_rows_for_olmc(olmc);
        for i in start * num_cols .. end * num_cols {
            self.fuses[i] = false;
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        assert!(self.chip == Chip::GAL16V8 || self.chip == Chip::GAL20V8);
        match mode {
        Mode::Mode1 => {
            self.syn = true;
            self.ac0 = false;
        }
        Mode::Mode2 => {
            self.syn = true;
            self.ac0 = true;
        }
        Mode::Mode3 => {
            self.syn = false;
            self.ac0 = true;
        }
        }
    }

    pub fn get_mode(&mut self) -> Mode {
        assert!(self.chip == Chip::GAL16V8 || self.chip == Chip::GAL20V8);
        match (self.syn, self.ac0) {
        (true, false) => Mode::Mode1,
        (true, true) => Mode::Mode2,
        (false, true) => Mode::Mode3,
        _ => panic!("Bad syn/ac0 mode"),
        }
    }
}
