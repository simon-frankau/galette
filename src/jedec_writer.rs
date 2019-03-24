extern crate itertools;

use self::itertools::Itertools;
use std::ffi::CStr;
use std::fs::File;
use std::io::Write;
use std::os::raw::c_char;

// IDs used in C.
const GAL16V8: i32 = 1;
const GAL20V8: i32 = 2;
const GAL22V10: i32 = 3;
const GAL20RA10: i32 = 4;

// Number of rows of fuses.
const ROW_COUNT_16V8: usize = 64;
const ROW_COUNT_20V8: usize = 64;
const ROW_COUNT_22V10: usize = 132;
const ROW_COUNT_20RA10: usize = 80;

// Number of fuses per-row.
const ROW_LEN_ADR16: usize = 32;
const ROW_LEN_ADR20: usize = 40;
const ROW_LEN_ADR22V10: usize = 44;
const ROW_LEN_ADR20RA10: usize = 40;

// Size of various other fields.
const SIG_SIZE: usize = 64;
const AC1_SIZE: usize = 8;
const PT_SIZE: usize = 64;

// Config use on the C side.
#[repr(C)]
#[derive(Debug)]
pub struct Config {
    gen_fuse: i16,
    gen_chip: i16,
    gen_pin: i16,
    jedec_sec_bit: i16,
    jedec_fuse_chk: i16,
}

#[no_mangle]
pub extern "C" fn write_jedec_c(
    file_name: *const c_char,
    gal_type: i32,
    config: *const Config,
    gal_fuses: *const u8,
    gal_xor: *const u8,
    gal_s1: *const u8,
    gal_sig: *const u8,
    gal_ac1: *const u8,
    gal_pt: *const u8,
    gal_syn: u8,
    gal_ac0: u8,
) {
    let xor_size = match gal_type {
        GAL16V8 => 8,
        GAL20V8 => 8,
        GAL22V10 => 10,
        GAL20RA10 => 10,
        _ => panic!("Nope"),
    };

    let fuse_size = match gal_type {
        GAL16V8 => ROW_LEN_ADR16 * ROW_COUNT_16V8,
        GAL20V8 => ROW_LEN_ADR20 * ROW_COUNT_20V8,
        GAL22V10 => ROW_LEN_ADR22V10 * ROW_COUNT_22V10,
        GAL20RA10 => ROW_LEN_ADR20RA10 * ROW_COUNT_20RA10,
        _ => panic!("Nope"),
    };

    unsafe {
        let file_name_rs = CStr::from_ptr(file_name);

        let str = make_jedec(
            gal_type,
            &(*config),
            std::slice::from_raw_parts(gal_fuses, fuse_size),
            std::slice::from_raw_parts(gal_xor, xor_size),
            std::slice::from_raw_parts(gal_s1, 10),
            std::slice::from_raw_parts(gal_sig, SIG_SIZE),
            std::slice::from_raw_parts(gal_ac1, AC1_SIZE),
            std::slice::from_raw_parts(gal_pt, PT_SIZE),
            gal_syn,
            gal_ac0,
        );

        let mut file = File::create(file_name_rs.to_str().unwrap()).unwrap();
        file.write_all(str.as_bytes());
    }
}

////////////////////////////////////////////////////////////////////////
// Structure to track the fuse checksum.

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

    fn add(&mut self, bit: u8) {
        if bit != 0 {
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

    fn add(&mut self, data: &[u8]) {
        self.add_iter(data.iter());
    }

    fn add_iter<'b, I>(&mut self, data: I)
        where I: Iterator<Item = &'b u8> {
        self.buf.push_str(&format!("*L{:04} ", self.idx));
        for bit in data {
            self.buf.push_str(if *bit != 0 { "1" } else { "0" });
            self.checksum.add(*bit);
            self.idx += 1;
        }
        self.buf.push('\n');
    }

    // Skip over zeros, updating count and checksum.
    fn skip_iter<'b, I>(&mut self, data: I)
        where I: Iterator<Item = &'b u8> {
        for _bit in data {
            self.checksum.add(*_bit); // (It's a zero.)
            self.idx += 1;
        }
    }

    fn checksum(&mut self) {
        self.buf.push_str(&format!("*C{:04x}\n", self.checksum.get()));
    }
}

////////////////////////////////////////////////////////////////////////
// Core function to generate a string of the JEDEC file, given the
// config, fuses, etc.
//
// It's galasm-compatible.

fn make_jedec(
    gal_type: i32,
    config: &Config,
    gal_fuses: &[u8],
    gal_xor: &[u8],
    gal_s1: &[u8],
    gal_sig: &[u8],
    gal_ac1: &[u8],
    gal_pt: &[u8],
    gal_syn: u8,
    gal_ac0: u8,
) -> String {
    let row_len = match gal_type {
        GAL16V8   => ROW_LEN_ADR16,
        GAL20V8   => ROW_LEN_ADR20,
        GAL22V10  => ROW_LEN_ADR22V10,
        GAL20RA10 => ROW_LEN_ADR20RA10,
        _ => panic!("Nope"),
    };

    let mut buf = String::new();

    buf.push_str("\x02\n");

    // TODO: Backwards compatibility.
    buf.push_str("Used Program:   GALasm 2.1\n");
    buf.push_str("GAL-Assembler:  GALasm 2.1\n");
    buf.push_str(match gal_type {
        GAL16V8   => "Device:         GAL16V8\n\n",
        GAL20V8   => "Device:         GAL20V8\n\n",
        GAL22V10  => "Device:         GAL22V10\n\n",
        GAL20RA10 => "Device:         GAL20RA10\n\n",
        _ => panic!("Nope"),
    });

    // Default value of gal_fuses
    buf.push_str("*F0\n");

    // Security bit state.
    buf.push_str(if config.jedec_sec_bit != 0 {
        "*G1\n"
    } else {
        "*G0\n"
    });

    // Number of fuses.
    // TODO: Should be calculated.
    buf.push_str(match gal_type {
        GAL16V8   => "*QF2194\n",
        GAL20V8   => "*QF2706\n",
        GAL22V10  => "*QF5892\n",
        GAL20RA10 => "*QF3274\n",
        _ => panic!("Nope"),
    });

    {
        // Construct fuse matrix.
        let mut fuse_builder = FuseBuilder::new(&mut buf);

        // Break the fuse map into chunks representing rows.
        for row in &gal_fuses.iter().chunks(row_len) {
            let (mut check_iter, mut print_iter) = row.tee();

            // Only write out non-zero bits.
            if check_iter.any(|x| *x != 0) {
                fuse_builder.add_iter(print_iter);
            } else {
                // Process the bits without writing.
                fuse_builder.skip_iter(print_iter);
            }
        }

        // XOR bits are interleaved with S1 bits on GAL22V10.
        if gal_type != GAL22V10 {
            fuse_builder.add(gal_xor)
        } else {
            let bits = itertools::interleave(gal_xor.iter(), gal_s1.iter());
            fuse_builder.add_iter(bits);
        }

        fuse_builder.add(gal_sig);

        if (gal_type == GAL16V8) || (gal_type == GAL20V8) {
            fuse_builder.add(gal_ac1);
            fuse_builder.add(gal_pt);
            fuse_builder.add(&[gal_syn]);
            fuse_builder.add(&[gal_ac0]);
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
