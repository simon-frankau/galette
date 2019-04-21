extern crate itertools;

use chips::Chip;
use gal::GAL;
use self::itertools::Itertools;

#[derive(Debug)]
pub struct Config {
    pub gen_fuse: bool,
    pub gen_chip: bool,
    pub gen_pin: bool,
    pub jedec_sec_bit: bool,
}

////////////////////////////////////////////////////////////////////////
// Structure to track the JEDEC fuse checksum.

struct CheckSummer {
    bit_num: u8,
    byte: u8,
    sum: u16,
}

impl CheckSummer {
    fn new() -> Self {
        CheckSummer {
            bit_num: 0,
            byte: 0,
            sum: 0,
        }
    }

    fn add(&mut self, bit: bool) {
        if bit {
            self.byte |= 1 << self.bit_num
        };
        self.bit_num += 1;
        if self.bit_num == 8 {
            self.sum = self.sum.wrapping_add(self.byte as u16);
            self.byte = 0;
            self.bit_num = 0;
        }
    }

    fn get(&self) -> u16 {
        (self.sum + self.byte as u16) & 0xffff
    }
}

////////////////////////////////////////////////////////////////////////
// A helper to write fuse entries into the buffer for the given bits,
// updating the offset and the checksum as we go.

struct FuseBuilder<'a> {
    buf: &'a mut String,
    checksum: CheckSummer,
    idx: usize,
}

impl<'a> FuseBuilder<'a> {
    fn new(buf: &mut String) -> FuseBuilder {
        FuseBuilder {
            buf: buf,
            checksum: CheckSummer::new(),
            idx: 0,
        }
    }

    fn add(&mut self, data: &[bool]) {
        self.add_iter(data.iter());
    }

    fn add_iter<'b, I>(&mut self, data: I)
    where
        I: Iterator<Item = &'b bool>,
    {
        self.buf.push_str(&format!("*L{:04} ", self.idx));
        for bit in data {
            self.buf.push_str(if *bit { "1" } else { "0" });
            self.checksum.add(*bit);
            self.idx += 1;
        }
        self.buf.push('\n');
    }

    // Skip over zeros, updating count and checksum.
    fn skip_iter<'b, I>(&mut self, data: I)
    where
        I: Iterator<Item = &'b bool>,
    {
        for _bit in data {
            self.checksum.add(*_bit); // (It's a zero.)
            self.idx += 1;
        }
    }

    fn checksum(&mut self) {
        self.buf
            .push_str(&format!("*C{:04x}\n", self.checksum.get()));
    }
}

////////////////////////////////////////////////////////////////////////
// Core function to generate a string of the JEDEC file, given the
// config, fuses, etc.
//
// It's galasm-compatible.

pub fn make_jedec(
    config: &Config,
    gal: &GAL,
) -> String {
    let chip = gal.chip;
    let row_len = chip.num_cols();

    let mut buf = String::new();

    buf.push_str("\x02\n");

    // TODO: Backwards compatibility.
    buf.push_str("Used Program:   GALasm 2.1\n");
    buf.push_str("GAL-Assembler:  GALasm 2.1\n");
    buf.push_str(&format!("Device:         {}\n\n", chip.name()));
    // Default value of gal_fuses
    buf.push_str("*F0\n");

    // Security bit state.
    buf.push_str(if config.jedec_sec_bit {
        "*G1\n"
    } else {
        "*G0\n"
    });

    // Number of fuses.
    buf.push_str(&format!("*QF{}\n", chip.total_size()));

    {
        // Construct fuse matrix.
        let mut fuse_builder = FuseBuilder::new(&mut buf);

        // Break the fuse map into chunks representing rows.
        for row in &gal.fuses.iter().chunks(row_len) {
            let (mut check_iter, mut print_iter) = row.tee();

            // Only write out non-zero bits.
            if check_iter.any(|x| *x) {
                fuse_builder.add_iter(print_iter);
            } else {
                // Process the bits without writing.
                fuse_builder.skip_iter(print_iter);
            }
        }

        // XOR bits are interleaved with S1 bits on GAL22V10 (stored
        // in the 'ac1' field, as it's the same function).
        if chip != Chip::GAL22V10 {
            fuse_builder.add(&gal.xor)
        } else {
            let bits = itertools::interleave(gal.xor.iter(), gal.ac1.iter());
            fuse_builder.add_iter(bits);
        }

        fuse_builder.add(&gal.sig);

        if (chip == Chip::GAL16V8) || (chip == Chip::GAL20V8) {
            fuse_builder.add(&gal.ac1);
            fuse_builder.add(&gal.pt);
            fuse_builder.add(&[gal.syn]);
            fuse_builder.add(&[gal.ac0]);
        }

        // Fuse checksum.
        fuse_builder.checksum();
    }

    buf.push_str("*\n");
    buf.push('\x03');

    // TODO: This should be a 16-bit checksum, but galasm does *not*
    // do that. Standard says modulo 65535, a la TCP/IP, need to check
    // what reading tools do.
    let file_checksum = buf.as_bytes().iter().map(|c| *c as u32).sum::<u32>();
    buf.push_str(&format!("{:04x}\n", file_checksum));

    return buf;
}
