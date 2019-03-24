use std::ffi::CStr;
use std::fs::File;
use std::io::Write;
use std::os::raw::c_char;

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
    unsafe {
        let file_name_rs = CStr::from_ptr(file_name);

        let str = make_jedec(
            gal_type,
            &(*config),
            std::slice::from_raw_parts(gal_fuses, 5808),
            std::slice::from_raw_parts(gal_xor, 10),
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

const GAL16V8: i32 = 1;
const GAL20V8: i32 = 2;
const GAL22V10: i32 = 3;
const GAL20RA10: i32 = 4;

const ROW_SIZE_16V8: i32 = 64;
const ROW_SIZE_20V8: i32 = 64;
const ROW_SIZE_22V10: i32 = 132;
const ROW_SIZE_20RA10: i32 = 80;

const MAX_FUSE_ADR16: i32 = 31;
const MAX_FUSE_ADR20: i32 = 39;
const MAX_FUSE_ADR22V10: i32 = 43;
const MAX_FUSE_ADR20RA10: i32 = 39;

const SIG_SIZE: usize = 64;
const AC1_SIZE: usize = 8;
const PT_SIZE: usize = 64;

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
            self.sum = (self.sum + self.byte as u16) & 0xffff;
            self.byte = 0;
            self.bit_num = 0;
        }
    }

    fn get(&self) -> u16 {
        (self.sum + self.byte as u16) & 0xffff
    }
}

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
    let (max_fuse_addr, row_size, xor_size) = match gal_type {
        GAL16V8 => (MAX_FUSE_ADR16, ROW_SIZE_16V8, 8),
        GAL20V8 => (MAX_FUSE_ADR20, ROW_SIZE_20V8, 8),
        GAL22V10 => (MAX_FUSE_ADR22V10, ROW_SIZE_22V10, 10),
        GAL20RA10 => (MAX_FUSE_ADR20RA10, ROW_SIZE_20RA10, 10),
        _ => panic!("Nope"),
    };

    let mut buf = String::new();

    buf.push_str("\x02\n");

    // TODO: Backwards compatibility.
    buf.push_str("Used Program:   GALasm 2.1\n");
    buf.push_str("GAL-Assembler:  GALasm 2.1\n");
    buf.push_str(match gal_type {
        GAL16V8 => "Device:         GAL16V8\n\n",
        GAL20V8 => "Device:         GAL20V8\n\n",
        GAL22V10 => "Device:         GAL22V10\n\n",
        GAL20RA10 => "Device:         GAL20RA10\n\n",
        _ => panic!("Nope"),
    });
    // Default value of gal_fuses
    buf.push_str("*F0\n");
    buf.push_str(if config.jedec_sec_bit != 0 {
        "*G1\n"
    } else {
        "*G0\n"
    });
    buf.push_str(match gal_type {
        GAL16V8 => "*QF2194\n",
        GAL20V8 => "*QF2706\n",
        GAL22V10 => "*QF5892\n",
        GAL20RA10 => "*QF3274\n",
        _ => panic!("Nope"),
    });

    // Construct fuse matrix.
    let mut bitnum = 0;
    let mut bitnum2 = 0;
    let mut flag = 0;
    let mut checksum = CheckSummer::new();

    for m in 0..row_size {
        flag = 0;
        bitnum2 = bitnum;

        // Find the first non-zero bit.
        for n in 0..max_fuse_addr + 1 {
            if gal_fuses[bitnum2] != 0 {
                flag = 1;
                break;
            }
            bitnum2 += 1;
        }

        if flag != 0 {
            buf.push_str(&format!("*L{:04} ", bitnum));

            for n in 0..max_fuse_addr + 1 {
                buf.push_str(if gal_fuses[bitnum] != 0 { "1" } else { "0" });
                bitnum += 1;
            }

            buf.push_str("\n");
        } else {
            bitnum = bitnum2;
        }
    }

    if flag == 0 {
        bitnum = bitnum2;
    }

    for n in 0..(((max_fuse_addr + 1) * row_size) as usize) {
        checksum.add(gal_fuses[n]);
    }

    // XOR bits
    buf.push_str(&format!("*L{:04} ", bitnum));

    for n in 0..xor_size {
        buf.push_str(if gal_xor[n] != 0 { "1" } else { "0" });
        checksum.add(gal_xor[n]);
        bitnum += 1;

        if gal_type == GAL22V10 {
            // S1 of 22V10
            buf.push_str(if gal_s1[n] != 0 { "1" } else { "0" });
            checksum.add(gal_s1[n]);
            bitnum += 1;
        }
    }
    buf.push('\n');

    // Signature
    buf.push_str(&format!("*L{:04} ", bitnum));
    for n in 0..SIG_SIZE {
        buf.push_str(if gal_sig[n] != 0 { "1" } else { "0" });
        checksum.add(gal_sig[n]);
        bitnum += 1;
    }
    buf.push('\n');

    if (gal_type == GAL16V8) || (gal_type == GAL20V8) {
        // AC1 bits
        buf.push_str(&format!("*L{:04} ", bitnum));
        for n in 0..AC1_SIZE {
            buf.push_str(if gal_ac1[n] != 0 { "1" } else { "0" });
            checksum.add(gal_ac1[n]);
            bitnum += 1;
        }
        buf.push('\n');

        // PT bits
        buf.push_str(&format!("*L{:04} ", bitnum));
        for n in 0..PT_SIZE {
            buf.push_str(if gal_pt[n] != 0 { "1" } else { "0" });
            checksum.add(gal_pt[n]);
            bitnum += 1;
        }
        buf.push('\n');

        // SYN bit
        buf.push_str(&format!("*L{:04} ", bitnum));
        buf.push_str(if gal_syn != 0 { "1" } else { "0" });
        checksum.add(gal_syn);
        buf.push('\n');
        bitnum += 1;

        // AC0 bit
        buf.push_str(&format!("*L{:04} ", bitnum));
        buf.push_str(if gal_ac0 != 0 { "1" } else { "0" });
        checksum.add(gal_ac0);
        buf.push('\n');
    }

    buf.push_str(&format!("*C{:04x}\n", checksum.get()));

    // Closing asterisk
    buf.push_str("*\n");

    buf.push('\x03');

    // TODO: This should be a 16-bit checksum, but galasm does *not*
    // do that. Standard says modulo 65535, a la TCP/IP, need to check
    // what reading tools do.
    let file_checksum = buf.as_bytes().iter().map(|c| *c as u32).sum::<u32>();
    buf.push_str(&format!("{:04x}\n", file_checksum));

    return buf;
}
