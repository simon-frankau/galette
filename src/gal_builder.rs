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
