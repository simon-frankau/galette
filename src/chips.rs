
#[derive(PartialEq,Clone,Copy)]
pub enum Chip {
    GAL16V8,
    GAL20V8,
    GAL22V10,
    GAL20RA10,
}

struct ChipData {
    // Size of the main fuse array
    num_rows: usize,
    num_cols: usize,
    xor_size: usize,
}

const GAL16V8_DATA: ChipData = ChipData {
    num_rows: 64,
    num_cols: 32,
    xor_size: 8,
};

const GAL20V8_DATA: ChipData = ChipData {
    num_rows: 64,
    num_cols: 40,
    xor_size: 8,
};

const GAL22V10_DATA: ChipData = ChipData {
    num_rows: 132,
    num_cols: 44,
    xor_size: 10,
};

const GAL20RA10_DATA: ChipData = ChipData {
    num_rows: 80,
    num_cols: 40,
    xor_size: 10,
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

    pub fn xor_size(&self) -> usize {
        self.get_chip_data().xor_size
    }
}