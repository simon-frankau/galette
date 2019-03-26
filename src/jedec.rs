use chips::Chip;

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

// This structure is passed across the C boundary, so let's be careful.
const MAGIC: i32 = 0x12345678;

impl Jedec {
    pub fn new(gal_type: Chip) -> Jedec {

        let fuse_size = gal_type.logic_size();
        let num_olmcs = gal_type.num_olmcs();

        Jedec {
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
}
