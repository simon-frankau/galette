use chips::Chip;
use jedec::Jedec;
use olmc;
use olmc::OLMC;

#[no_mangle]
pub extern "C" fn set_and_c(
    jedec: *mut ::jedec::Jedec,
    row: u32,
    pin_num: u32,
    negation: u32,
) -> i32 {
    let mut jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.check_magic();

    match set_and(
        &mut jedec,
        row as usize,
        pin_num as usize,
        negation != 0,
    ) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

// Add an 'and' term to a fuse map.
fn set_and(
    jedec: &mut Jedec,
    row: usize,
    pin_num: usize,
    negation: bool,
) -> Result<(), String> {
    let chip = jedec.chip;
    let row_len = chip.num_cols();
    let column = jedec.pin_to_column(pin_num)?;

    // Is it a registered OLMC pin?
    // If yes, then correct the negation.
    let mut neg_off = if negation { 1 } else { 0 };
    if chip == Chip::GAL22V10 && (pin_num >= 14 && pin_num <= 23) && !jedec.s1[23 - pin_num] {
        neg_off = 1 - neg_off;
    }

    jedec.fuses[row * row_len + column + neg_off] = false;
    Ok(())
}

pub fn set_unused(
    jedec: &mut Jedec,
    olmcs: &[OLMC]
) -> Result<(), usize> {

    // NB: Length of num_olmcs may be incorrect because that includes AR, SP, etc.
    for i in 0..jedec.chip.num_olmcs() {
        if olmcs[i].pin_type == olmc::NOTUSED || olmcs[i].pin_type == olmc::INPUT {
            jedec.clear_olmc(i);
        }
    }

    // Special cases
    match jedec.chip {
        Chip::GAL22V10 => {
            if olmcs[10].pin_type == 0    /* set row of AR equal 0 */ {
                jedec.clear_olmc(10);
            }
            if olmcs[11].pin_type == 0    /* set row of SP equal 0 */ {
                jedec.clear_olmc(11);
            }
        }
        Chip::GAL20RA10 => {
            for i in 0..jedec.chip.num_olmcs() {
                if olmcs[i].pin_type != olmc::NOTUSED {
                    if olmcs[i].pin_type == olmc::REGOUT && olmcs[i].clock == 0{
                        // return Err(format?("missing clock definition (.CLK) of registered output on pin {}", n + 14));
                        return Err(i + 14);
                    }

                    if olmcs[i].clock == 0 {
                        let start_row = jedec.chip.start_row_for_olmc(i);
                        jedec.clear_row(start_row, 1);
                    }

                    if olmcs[i].pin_type == olmc::REGOUT {
                        if olmcs[i].arst == olmc::NOTUSED {
                            let start_row = jedec.chip.start_row_for_olmc(i);
                            jedec.clear_row(start_row, 2);
                        }

                        if olmcs[i].aprst == olmc::NOTUSED {
                            let start_row = jedec.chip.start_row_for_olmc(i);
                            jedec.clear_row(start_row, 3);
                        }
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}

// TODO!
const MODE1: i32 =           1;               /* modes (SYN, AC0) */
const MODE2: i32 =           2;
const MODE3: i32 =           3;

const SUFFIX_NON: i32 =              0;	/* possible suffixes */
const SUFFIX_T: i32 =                1;
const SUFFIX_R: i32 =                2;
const SUFFIX_E: i32 =                3;
const SUFFIX_CLK: i32 =              4;
const SUFFIX_APRST: i32 =            5;
const SUFFIX_ARST: i32 =             6;


pub fn get_bounds(
    jedec: &Jedec,
    act_olmc: usize,
    olmcs: &[OLMC],
    suffix: i32,
    mode: i32,
) -> (usize, usize, usize) {
    let start_row = jedec.chip.start_row_for_olmc(act_olmc);
    let mut max_row = jedec.chip.num_rows_for_olmc(act_olmc);
    let mut row_offset = 0;

    match jedec.chip {
        Chip::GAL16V8 | Chip::GAL20V8 => {
            if suffix == SUFFIX_E {/* when tristate control use */
                row_offset = 0; /* first row (=> offset = 0) */
                max_row = 1;
            } else if mode != MODE1 && olmcs[act_olmc].pin_type != olmc::REGOUT {
                row_offset = 1; /* then init. row-offset */
            }
        }
        Chip::GAL22V10 => {
            if suffix == SUFFIX_E { /* enable is the first row */
                row_offset = 0; /* of the OLMC             */
                max_row = 1;
            } else {
                if act_olmc == 10 || act_olmc == 11 {
                    row_offset = 0; /* AR, SP?, then no offset */
                    max_row = 1;
                } else {
                    row_offset = 1; /* second row => offset = 1 */
                }
            }
        }
        Chip::GAL20RA10 => {
            match suffix {
                SUFFIX_E => { /* enable is the first row */
                    row_offset = 0; /* of the OLMC             */
                    max_row = 1;
                }
                SUFFIX_CLK => { /* Clock is the second row */
                    row_offset = 1; /* of the OLMC             */
                    max_row = 2;
                }
                SUFFIX_ARST => { /* AReset is the third row */
                    row_offset = 2; /* of the OLMC             */
                    max_row = 3;
                }
                SUFFIX_APRST => { /* APreset is the fourth row */
                    row_offset = 3; /* of the OLMC               */
                    max_row = 4;
                }
                _ => { /* output equation starts */
                    if row_offset <= 3 { /* at the fifth row       */
                        row_offset = 4;
                    }
                }
            }
        }
    }

    (start_row, max_row, row_offset)
}
