use chips::Chip;
use gal_builder::Pin;

pub struct Jedec {
    pub chip: Chip,
    pub fuses: Vec<bool>,
    pub xor: Vec<bool>,
    pub sig: Vec<bool>,
    pub ac1: Vec<bool>,
    pub pt: Vec<bool>,
    pub syn: bool,
    pub ac0: bool,
    pub s1: Vec<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub start_row: usize,
    pub max_row: usize,
    pub row_offset: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Term {
    pub line_num: i32,
    pub rhs: Vec<Pin>,
    pub ops: Vec<i8>,
}

// Mode enums for the v8s
#[derive(PartialEq,Clone,Copy)]
pub enum Mode {
    Mode1,
    Mode2,
    Mode3,
}

// Map pin number to column within the fuse table. The mappings depend
// on the mode settings for the v8s, so they're here rather than in
// chips.rs. -1 if it can't be used.

// GAL16V8
const PIN_TO_COL_16_MODE1: [i32; 20] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, -1, 30, 26, 22, 18, -1, -1, 14, 10, 6, -1,
];
const PIN_TO_COL_16_MODE2: [i32; 20] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, -1, 30, -1, 26, 22, 18, 14, 10, 6, -1, -1,
];
const PIN_TO_COL_16_MODE3: [i32; 20] = [
    -1, 0, 4, 8, 12, 16, 20, 24, 28, -1, -1, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

// GAL20V8
const PIN_TO_COL_20_MODE1: [i32; 24] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, 38, 34, 30, 26, 22, -1, -1, 18, 14, 10, 6, -1,
];
const PIN_TO_COL_20_MODE2: [i32; 24] = [
    2, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, 38, 34, -1, 30, 26, 22, 18, 14, 10, -1, 6, -1,
];
const PIN_TO_COL_20_MODE3: [i32; 24] = [
    -1, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, -1, 38, 34, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

// GAL22V10
const PIN_TO_COL_22V10: [i32; 24] = [
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, -1, 42, 38, 34, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];

// GAL20RA10
const PIN_TO_COL_20RA10: [i32; 24] = [
    -1, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, -1, -1, 38, 34, 30, 26, 22, 18, 14, 10, 6, 2, -1,
];


impl Jedec {
    pub fn new(gal_type: Chip) -> Jedec {

        let fuse_size = gal_type.logic_size();
        let num_olmcs = gal_type.num_olmcs();

        Jedec {
            chip: gal_type,
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

    pub fn clear_row(&mut self, start_row: usize, row_offset: usize) {
        let num_cols = self.chip.num_cols();
        let start = (start_row + row_offset) * num_cols;
        for i in start .. start + num_cols {
            self.fuses[i] = false;
        }
    }

    fn clear_rows(&mut self, start_row: usize, row_offset: usize, max_row: usize) {
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

    pub fn get_mode(&self) -> Mode {
        assert!(self.chip == Chip::GAL16V8 || self.chip == Chip::GAL20V8);
        match (self.syn, self.ac0) {
        (true, false) => Mode::Mode1,
        (true, true) => Mode::Mode2,
        (false, true) => Mode::Mode3,
        _ => panic!("Bad syn/ac0 mode"),
        }
    }

    fn pin_to_column(&self, pin_num: usize) -> Result<usize, String> {
        let column_lookup: &[i32] = match self.chip {
            Chip::GAL16V8 => match self.get_mode() {
                Mode::Mode1 => &PIN_TO_COL_16_MODE1,
                Mode::Mode2 => &PIN_TO_COL_16_MODE2,
                Mode::Mode3 => &PIN_TO_COL_16_MODE3,
            },
            Chip::GAL20V8 => match self.get_mode() {
                Mode::Mode1 => &PIN_TO_COL_20_MODE1,
                Mode::Mode2 => &PIN_TO_COL_20_MODE2,
                Mode::Mode3 => &PIN_TO_COL_20_MODE3,
            },
            Chip::GAL22V10 => &PIN_TO_COL_22V10,
            Chip::GAL20RA10 => &PIN_TO_COL_20RA10,
        };

        let column = column_lookup[pin_num - 1];

        if column < 0 {
                // TODO: Better error stuff.
                return Err::<usize, String>(format!("{} cannot use pin {} as input or feedback",
                            self.name_for_error(),
                            pin_num));
        }

        Ok(column as usize)
    }

    fn name_for_error(&self) -> &str {
        match self.chip {
            Chip::GAL16V8 => match self.get_mode() {
                Mode::Mode1 => "GAL16V8 (mode 1)",
                Mode::Mode2 => "GAL16V8 (mode 2)",
                Mode::Mode3 => "GAL16V8 (mode 3)",
            },
            Chip::GAL20V8 => match self.get_mode() {
                Mode::Mode1 => "GAL20V8 (mode 1)",
                Mode::Mode2 => "GAL20V8 (mode 2)",
                Mode::Mode3 => "GAL20V8 (mode 3)",
            },
            Chip::GAL22V10 => "GAL22V10",
            Chip::GAL20RA10 => "GAL20RA10",
        }
    }

    // Add an 'and' term to a fuse map.
    fn set_and(
        &mut self,
        row: usize,
        pin_num: usize,
        negation: bool,
    ) -> Result<(), i32> {
        let chip = self.chip;
        let row_len = chip.num_cols();
        let column = match self.pin_to_column(pin_num) {
            Ok(x) => x,
            Err(_) => return Err(26),
        };

        // Is it a registered OLMC pin?
        // If yes, then correct the negation.
        let mut neg_off = if negation { 1 } else { 0 };
        if chip == Chip::GAL22V10 && (pin_num >= 14 && pin_num <= 23) && !self.s1[23 - pin_num] {
            neg_off = 1 - neg_off;
        }

        self.fuses[row * row_len + column + neg_off] = false;
        Ok(())
    }

    pub fn add_term(
        &mut self,
        term: &Term,
        bounds: &Bounds,
    ) -> Result<(), i32> {
        let mut bounds = *bounds;
        let rhs = &term.rhs;
        let ops = &term.ops;
        // if GND, set row equal 0
        if rhs.len() == 1 && (rhs[0].pin as usize == self.chip.num_pins() || rhs[0].pin as usize == self.chip.num_pins() / 2) {
            if rhs[0].neg != 0 {
                // /VCC and /GND are not allowed
                return Err(term.line_num * 0x10000 + 25);
            }

            if rhs[0].pin as usize == self.chip.num_pins() / 2 {
                self.clear_row(bounds.start_row, bounds.row_offset);
            }
        } else {
            for i in 0..rhs.len() {
                let pin_num = rhs[i].pin;

                if pin_num as usize == self.chip.num_pins() || pin_num as usize == self.chip.num_pins() / 2 {
                    return Err(term.line_num * 0x10000 + 28);
                }

                if ops[i] == 43 || ops[i] == 35 {
                    // If an OR, go to next row.
                    bounds.row_offset += 1;

                    if bounds.row_offset == bounds.max_row {
                        // too many ORs?
                        return Err(term.line_num * 0x10000 + 30);
                    }
                }

                // Set ANDs.
                if let Err(i) =  self.set_and(bounds.start_row + bounds.row_offset, pin_num as usize, rhs[i].neg != 0) {
                    return Err(term.line_num * 0x10000 + i);
                }
            }
        }

        // Then zero the rest...
        bounds.row_offset += 1;
        self.clear_rows(bounds.start_row, bounds.row_offset, bounds.max_row);

        Ok(())
    }
}
