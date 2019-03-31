#[derive(PartialEq,Clone,Copy)]
pub enum Chip {
    GAL16V8,
    GAL20V8,
    GAL22V10,
    GAL20RA10,
}

// Number of rows for each OLMC in the 22V10's fuse table (only 22V10
// is non-uniform).
//
// The last two OLMCs aren't connected to pins, but represent SP
// and AR.
const OLMC_SIZE_22V10: [i32; 12] = [9, 11, 13, 15, 17, 17, 15, 13, 11, 9, 1, 1];
// And for everything else...
const OLMC_SIZE_DEFAULT: i32 = 8;

// Map OLMC number to starting row within the fuse table
const OLMC_ROWS_XXV8: [i32; 8] = [56, 48, 40, 32, 24, 16, 8, 0];
const OLMC_ROWS_22V10: [i32; 12] = [122, 111, 98, 83, 66, 49, 34, 21, 10, 1, 0, 131];
const OLMC_ROWS_20RA10: [i32; 10] = [72, 64, 56, 48, 40, 32, 24, 16, 8, 0];

struct ChipData {
    name: &'static str,
    // Size of the main fuse array.
    num_rows: usize,
    num_cols: usize,
    // Total size of the bitstream.
    // TODO: Should be calculated.
    total_size: usize,
    // Range of pins that are backed by OLMCs
    min_olmc_pin: usize,
    max_olmc_pin: usize,
    // Mapping from OLMC number to starting row in the fuse grid.
    olmc_map: &'static [i32],
}

const GAL16V8_DATA: ChipData = ChipData {
    name: "GAL16V8",
    num_rows: 64,
    num_cols: 32,
    total_size: 2194,
    min_olmc_pin: 12,
    max_olmc_pin: 19,
    olmc_map: &OLMC_ROWS_XXV8,
};

const GAL20V8_DATA: ChipData = ChipData {
    name: "GAL20V8",
    num_rows: 64,
    num_cols: 40,
    total_size: 2706,
    min_olmc_pin: 15,
    max_olmc_pin: 22,
    olmc_map: &OLMC_ROWS_XXV8,
};

const GAL22V10_DATA: ChipData = ChipData {
    name: "GAL22V10",
    num_rows: 132,
    num_cols: 44,
    total_size: 5892,
    min_olmc_pin: 14,
    max_olmc_pin: 23,
    olmc_map: &OLMC_ROWS_22V10,
};

const GAL20RA10_DATA: ChipData = ChipData {
    name: "GAL20RA10",
    num_rows: 80,
    num_cols: 40,
    total_size: 3274,
    min_olmc_pin: 14,
    max_olmc_pin: 23,
    olmc_map: &OLMC_ROWS_20RA10,
};

impl Chip {
    fn get_chip_data(&self) -> &ChipData {
        match self {
            Chip::GAL16V8 => &GAL16V8_DATA,
            Chip::GAL20V8 => &GAL20V8_DATA,
            Chip::GAL22V10 => &GAL22V10_DATA,
            Chip::GAL20RA10 => &GAL20RA10_DATA,
        }
    }

    pub fn name(&self) -> &str {
        self.get_chip_data().name
    }

    pub fn num_rows(&self) -> usize {
        self.get_chip_data().num_rows
    }

    pub fn num_cols(&self) -> usize {
        self.get_chip_data().num_cols
    }

    pub fn logic_size(&self) -> usize {
        let data = self.get_chip_data();
        data.num_rows * data.num_cols
    }

    pub fn total_size(&self) -> usize {
        self.get_chip_data().total_size
    }

    pub fn pin_to_olmc(&self, pin: usize) -> Option<usize> {
        let data = self.get_chip_data();
        if data.min_olmc_pin <= pin && pin <= data.max_olmc_pin {
            Some(pin - data.min_olmc_pin)
        } else {
            None
        }
    }

    // Pin number of last OLMC'd output.
    pub fn last_olmc(&self) -> usize {
        self.get_chip_data().max_olmc_pin
    }

    // Count of OLMCs
    pub fn num_olmcs(&self) -> usize {
        let data = self.get_chip_data();
        data.max_olmc_pin - data.min_olmc_pin + 1
    }

    // First row number in the fuse table for the rows associated with an OLMC.
    pub fn start_row_for_olmc(&self, olmc_num: usize) -> usize {
        self.get_chip_data().olmc_map[olmc_num] as usize
    }

    // Not everything is easiest driven off a table...
    pub fn num_rows_for_olmc(&self, olmc_num: usize) -> usize {
        if *self == Chip::GAL22V10 {
            // Only 22V10 has non-uniform-sized OLMCs.
            OLMC_SIZE_22V10[olmc_num] as usize
        } else {
            OLMC_SIZE_DEFAULT as usize
        }
    }
}
