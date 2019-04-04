use chips::Chip;
use jedec::Jedec;
use jedec::Mode;
use olmc;
use olmc::OLMC;

#[repr(C)]
#[derive(Debug)]
pub struct Pin {
    neg: i8,
    pin: i8,
}

// Add an 'and' term to a fuse map.
fn set_and(
    jedec: &mut Jedec,
    row: usize,
    pin_num: usize,
    negation: bool,
) -> Result<(), i32> {
    let chip = jedec.chip;
    let row_len = chip.num_cols();
    let column = match jedec.pin_to_column(pin_num) {
        Ok(x) => x,
        Err(_) => return Err(26),
    };

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

// const SUFFIX_NON: i32 =              0;	/* possible suffixes */
// const SUFFIX_T: i32 =                1;
// const SUFFIX_R: i32 =                2;
const SUFFIX_E: i32 =                3;
const SUFFIX_CLK: i32 =              4;
const SUFFIX_APRST: i32 =            5;
const SUFFIX_ARST: i32 =             6;


pub fn get_bounds(
    jedec: &Jedec,
    act_olmc: usize,
    olmcs: &[OLMC],
    suffix: i32
) -> (usize, usize, usize) {
    let start_row = jedec.chip.start_row_for_olmc(act_olmc);
    let mut max_row = jedec.chip.num_rows_for_olmc(act_olmc);
    let mut row_offset = 0;

    match jedec.chip {
        Chip::GAL16V8 | Chip::GAL20V8 => {
            if suffix == SUFFIX_E {/* when tristate control use */
                row_offset = 0; /* first row (=> offset = 0) */
                max_row = 1;
            } else if jedec.get_mode() != Mode::Mode1 && olmcs[act_olmc].pin_type != olmc::REGOUT {
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

pub fn add_equation(
    jedec: &mut Jedec,
    olmcs: &[OLMC],
    _line_num: i32, // TODO
    lhs: &Pin,
    suffix: i32,
    rhs: &[Pin],
    ops: &[i8]
) -> Result<(), i32> {
    let act_olmc = jedec.chip.pin_to_olmc(lhs.pin as usize).unwrap();
    let (start_row, max_row, mut row_offset) = get_bounds(jedec, act_olmc, olmcs, suffix);

    // if GND, set row equal 0
    if rhs.len() == 1 && (rhs[0].pin as usize == jedec.chip.num_pins() || rhs[0].pin as usize == jedec.chip.num_pins() / 2) {
        if rhs[0].neg != 0 {
            // /VCC and /GND are not allowed
            return Err(25);
        }

        if rhs[0].pin as usize == jedec.chip.num_pins() / 2 {
            jedec.clear_row(start_row, row_offset);
        }
    } else {
        for i in 0..rhs.len() {
            let pin_num = rhs[i].pin;

            if pin_num as usize == jedec.chip.num_pins() || pin_num as usize == jedec.chip.num_pins() / 2 {
                return Err(28);
            }

            if ops[i] == 43 || ops[i] == 35 {
                // If an OR, go to next row.
                row_offset += 1;

                if row_offset == max_row {
                    // too many ORs?
                    return Err(30);
                }
            }

            // Set ANDs.
            set_and(jedec, start_row + row_offset, pin_num as usize, rhs[i].neg != 0)?;
        }
    }

    // Then zero the rest...
    row_offset += 1;
    jedec.clear_rows(start_row, row_offset, max_row);

    Ok(())
}