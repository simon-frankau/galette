//
// gal.rs: Fuse state
//
// The GAL structure holds the fuse state for a GAL. Some helper
// methods are provided to program sets of fuses, but the fuses can
// also be directly manipulated.
//

use chips::Chip;
use errors::Error;
use errors::ErrorCode;

pub use chips::Bounds;

// A 'Pin' represents an input to an equation - a potentially negated
// pin (represented by pin number).
//
// TODO: Use more appropriate types when C-interoperability goes away.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pin {
    pub neg: i8,
    pub pin: i8,
}

// A 'Term' represents a set of OR'd together sub-terms which are the
// ANDing of inputs and their negations. Special cases support
// true and false values (see 'true_term' and 'false_term' below.
//
// Terms are programmed into the GAL structure.
#[derive(Clone, Debug, PartialEq)]
pub struct Term {
    pub line_num: i32,
    // Each inner Vec represents an AND term. The overall term is the
    // OR of the inner terms.
    pub pins: Vec<Vec<Pin>>,
}

// The 'GAL' struct represents the fuse state of the GAL that we're
// going to program.
pub struct GAL {
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

// The GAL16V8 and GAL20V8 could run in one of three modes,
// interpreting the fuse array differently. This enum
// tracks the mode that's been set.
#[derive(PartialEq,Clone,Copy)]
pub enum Mode {
    // Combinatorial outputs
    Mode1,
    // Tristate outputs
    Mode2,
    // Tristate or registered outputs
    Mode3,
}

// Map input pin number to column within the fuse table. The mappings
// depend on the mode settings for the GALxxV8s, so they're here rather
// than in chips.rs. -1 if it can't be used.

// TODO: Perhaps encode errorr reasons for not being able to use pin,
// rather than just simply -1?

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

impl GAL {
    // Generate an empty fuse structure.
    pub fn new(gal_type: Chip) -> GAL {

        let fuse_size = gal_type.logic_size();
        let num_olmcs = gal_type.num_olmcs();

        GAL {
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

    // Set the fuses associated with mode for GALxxV8s.
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

    // Retrive the mode from the mode fuses.
    pub fn get_mode(&self) -> Mode {
        assert!(self.chip == Chip::GAL16V8 || self.chip == Chip::GAL20V8);
        match (self.syn, self.ac0) {
        (true, false) => Mode::Mode1,
        (true, true) => Mode::Mode2,
        (false, true) => Mode::Mode3,
        _ => panic!("Bad syn/ac0 mode"),
        }
    }

    // Enter a term into the given set of rows of the main logic array.
    pub fn add_term(
        &mut self,
        term: &Term,
        bounds: &Bounds,
    ) -> Result<(), Error> {
        let mut bounds = *bounds;
        for row in term.pins.iter() {
            if bounds.row_offset == bounds.max_row {
                // too many ORs?
                return Err(Error { code: ErrorCode::TOO_MANY_PRODUCTS, line: term.line_num });
            }

            for input in row.iter() {
                let pin_num = input.pin;

                // TODO: Should be part of set_and.
                if pin_num as usize == self.chip.num_pins() || pin_num as usize == self.chip.num_pins() / 2 {
                    return Err(Error { code: ErrorCode::BAD_POWER, line: term.line_num });
                }

                if let Err(code) = self.set_and(bounds.start_row + bounds.row_offset, pin_num as usize, input.neg != 0) {
                    return Err(Error { code: code, line: term.line_num });
                }
            }

            // Go to next row.
            bounds.row_offset += 1;
        }

        // Zero the unused part of the relevant space.
        self.clear_rows(&bounds);

        Ok(())
    }

    // Like add_term, but setting the term to false if no Term is provided.
    pub fn add_term_opt(
        &mut self,
        term: &Option<Term>,
        bounds: &Bounds,
    ) -> Result<(), Error> {
        match term {
            Some(term) => self.add_term(term, bounds),
            None => self.add_term(&false_term(0), bounds),
        }
    }

    // Clear out a set of rows, so they don't contribute to the term.
    fn clear_rows(&mut self, bounds: &Bounds) {
        let num_cols = self.chip.num_cols();
        let start = (bounds.start_row + bounds.row_offset) * num_cols;
        let end = (bounds.start_row + bounds.max_row) * num_cols;
        for i in start .. end {
            self.fuses[i] = false;
        }
    }

    // Map the input pin number to the fuse column number.
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

    // Get the name of the chip for error messages.
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

    // Add an 'AND' term to a fuse map.
    fn set_and(
        &mut self,
        row: usize,
        pin_num: usize,
        negation: bool,
    ) -> Result<(), ErrorCode> {
        let chip = self.chip;
        let row_len = chip.num_cols();
        let column = match self.pin_to_column(pin_num) {
            Ok(x) => x,
            Err(_) => return Err(ErrorCode::NOT_AN_INPUT),
        };

        // Is it a registered OLMC pin?
        // If yes, then correct the negation.
        // TODO: This feels pretty messy.
        let mut neg_off = if negation { 1 } else { 0 };
        if chip == Chip::GAL22V10 && (pin_num >= 14 && pin_num <= 23) && !self.s1[23 - pin_num] {
            neg_off = 1 - neg_off;
        }

        self.fuses[row * row_len + column + neg_off] = false;
        Ok(())
    }
}

// Basic terms
pub fn true_term(line_num: i32) -> Term {
    // Empty row is always true (being the AND of nothing).
    Term {
        line_num: line_num,
        pins: vec!(Vec::new()),
    }
}

pub fn false_term(line_num: i32) -> Term {
    // No rows is always false (being the OR of nothing).
    Term {
        line_num: line_num,
        pins: Vec::new(),
    }
}
