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

impl Jedec {
    pub fn new() -> Jedec {
        Jedec {
            magic: 0x87654321,
            fuses: vec![false; 5808],
            xor: vec![false; 10],
            sig: vec![false; 64],
            ac1: vec![false; 8],
            pt: vec![false; 64],
            syn: false,
            ac0: false,
            s1: vec![false; 10],
        }
    }

    pub fn check_magic(&self) {
        assert!(self.magic == 0x87654321);
    }
}
