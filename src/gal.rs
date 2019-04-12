//
// gal.rs: Fuse state
//
// The GAL structure holds the fuse state for a GAL. Some helper
// methods are provided to program sets of fuses, but the fuses can
// also be directly manipulated.
//

use chips::Chip;
use errors::at_line;
use errors::Error;
use errors::ErrorCode;

pub use chips::Bounds;

// A 'Pin' represents an input to an equation - a potentially negated
// pin (represented by pin number).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pin {
    pub pin: usize,
    pub neg: bool,
}

// A 'Term' represents a set of OR'd together sub-terms which are the
// ANDing of inputs and their negations. Special cases support
// true and false values (see 'true_term' and 'false_term' below.
//
// Terms are programmed into the GAL structure.
#[derive(Clone, Debug, PartialEq)]
pub struct Term {
    pub line_num: u32,
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
#[derive(PartialEq, Clone, Copy)]
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
// than in chips.rs.

const OUT: Result<i32, ErrorCode> = Err(ErrorCode::NotAnInput);
const PWR: Result<i32, ErrorCode> = Err(ErrorCode::BadPower);

// GAL16V8
const PIN_TO_COL_16_MODE1: [Result<i32, ErrorCode>; 20] = [
    Ok(2),  Ok(0),  Ok(4),  Ok(8),  Ok(12), Ok(16), Ok(20), Ok(24), Ok(28), PWR,
    Ok(30), Ok(26), Ok(22), Ok(18), OUT,    OUT,    Ok(14), Ok(10), Ok(6),  PWR,
];
const PIN_TO_COL_16_MODE2: [Result<i32, ErrorCode>; 20] = [
    Ok(2),  Ok(0), Ok(4),  Ok(8),  Ok(12), Ok(16), Ok(20), Ok(24), Ok(28), PWR,
    Ok(30), OUT,   Ok(26), Ok(22), Ok(18), Ok(14), Ok(10), Ok(6),  OUT,    PWR,
];
const PIN_TO_COL_16_MODE3: [Result<i32, ErrorCode>; 20] = [
    OUT, Ok(0),  Ok(4),  Ok(8),  Ok(12), Ok(16), Ok(20), Ok(24), Ok(28), PWR,
    OUT, Ok(30), Ok(26), Ok(22), Ok(18), Ok(14), Ok(10), Ok(6),  Ok(2),  PWR,
];

// GAL20V8
const PIN_TO_COL_20_MODE1: [Result<i32, ErrorCode>; 24] = [
    Ok(2),  Ok(0),  Ok(4),  Ok(8),  Ok(12), Ok(16), Ok(20), Ok(24), Ok(28), Ok(32), Ok(36), PWR,
    Ok(38), Ok(34), Ok(30), Ok(26), Ok(22), OUT,    OUT,    Ok(18), Ok(14), Ok(10), Ok(6),  PWR,
];
const PIN_TO_COL_20_MODE2: [Result<i32, ErrorCode>; 24] = [
    Ok(2),  Ok(0),  Ok(4), Ok(8),  Ok(12), Ok(16), Ok(20), Ok(24), Ok(28), Ok(32), Ok(36), PWR,
    Ok(38), Ok(34), OUT,   Ok(30), Ok(26), Ok(22), Ok(18), Ok(14), Ok(10), OUT,    Ok(6),  PWR,
];
const PIN_TO_COL_20_MODE3: [Result<i32, ErrorCode>; 24] = [
    OUT, Ok(0),  Ok(4),  Ok(8),  Ok(12), Ok(16), Ok(20), Ok(24), Ok(28), Ok(32), Ok(36), PWR,
    OUT, Ok(38), Ok(34), Ok(30), Ok(26), Ok(22), Ok(18), Ok(14), Ok(10), Ok(6),  Ok(2),  PWR,
];

// GAL22V10
const PIN_TO_COL_22V10: [Result<i32, ErrorCode>; 24] = [
    Ok(0),  Ok(4),  Ok(8),  Ok(12), Ok(16), Ok(20), Ok(24), Ok(28), Ok(32), Ok(36), Ok(40), PWR,
    Ok(42), Ok(38), Ok(34), Ok(30), Ok(26), Ok(22), Ok(18), Ok(14), Ok(10), Ok(6),  Ok(2),  PWR,
];

// GAL20RA10
const PIN_TO_COL_20RA10: [Result<i32, ErrorCode>; 24] = [
    OUT, Ok(0),  Ok(4),  Ok(8),  Ok(12), Ok(16), Ok(20), Ok(24), Ok(28), Ok(32), Ok(36), PWR,
    OUT, Ok(38), Ok(34), Ok(30), Ok(26), Ok(22), Ok(18), Ok(14), Ok(10), Ok(6),  Ok(2),  PWR,
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
    pub fn add_term(&mut self, term: &Term, bounds: &Bounds) -> Result<(), Error> {
        let mut bounds = *bounds;
        for row in term.pins.iter() {
            if bounds.row_offset == bounds.max_row {
                // too many ORs?
                return at_line(term.line_num, Err(ErrorCode::TooManyProducts));
            }

            for input in row.iter() {
                at_line(term.line_num, self.set_and(bounds.start_row + bounds.row_offset, input.pin, input.neg))?;
            }

            // Go to next row.
            bounds.row_offset += 1;
        }

        // Zero the unused part of the relevant space.
        self.clear_rows(&bounds);

        Ok(())
    }

    // Like add_term, but setting the term to false if no Term is provided.
    pub fn add_term_opt(&mut self, term: &Option<Term>, bounds: &Bounds) -> Result<(), Error> {
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
        for i in start..end {
            self.fuses[i] = false;
        }
    }

    // Map the input pin number to the fuse column number.
    fn pin_to_column(&self, pin_num: usize) -> Result<usize, ErrorCode> {
        let column_lookup: &[Result<i32, ErrorCode>] = match self.chip {
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

        let column = column_lookup[pin_num - 1]?;

        Ok(column as usize)
    }

    // Add an 'AND' term to a fuse map.
    fn set_and(&mut self, row: usize, pin_num: usize, negation: bool) -> Result<(), ErrorCode> {
        let chip = self.chip;
        let row_len = chip.num_cols();
        let column = self.pin_to_column(pin_num)?;

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
pub fn true_term(line_num: u32) -> Term {
    // Empty row is always true (being the AND of nothing).
    Term {
        line_num: line_num,
        pins: vec![Vec::new()],
    }
}

pub fn false_term(line_num: u32) -> Term {
    // No rows is always false (being the OR of nothing).
    Term {
        line_num: line_num,
        pins: Vec::new(),
    }
}
