use blueprint::Blueprint;
use chips::Chip;
use jedec::Jedec;
use jedec::Mode;
use olmc;
use olmc::OLMC;
use olmc::PinType;
use writer;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pin {
    pub neg: i8,
    pub pin: i8,
}

// Config use on the C side.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Equation {
    pub line_num: i32,
    pub lhs: Pin,
    pub suffix: i32,
    pub num_rhs: i32,
    pub rhs: *const Pin,
    pub ops: *const i8
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

pub const SUFFIX_NON: i32 =              0;	/* possible suffixes */
pub const SUFFIX_T: i32 =                1;
pub const SUFFIX_R: i32 =                2;
pub const SUFFIX_E: i32 =                3;
pub const SUFFIX_CLK: i32 =              4;
pub const SUFFIX_APRST: i32 =            5;
pub const SUFFIX_ARST: i32 =             6;

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
            } else if jedec.get_mode() != Mode::Mode1 && olmcs[act_olmc].pin_type != PinType::REGOUT {
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
    eqn: &Equation,
) -> Result<(), i32> {
    let rhs = unsafe { std::slice::from_raw_parts(eqn.rhs, eqn.num_rhs as usize) };
    let ops = unsafe { std::slice::from_raw_parts(eqn.ops, eqn.num_rhs as usize) };

    let act_olmc = jedec.chip.pin_to_olmc(eqn.lhs.pin as usize).unwrap();
    let (start_row, max_row, mut row_offset) = get_bounds(jedec, act_olmc, olmcs, eqn.suffix);

    // if GND, set row equal 0
    if rhs.len() == 1 && (rhs[0].pin as usize == jedec.chip.num_pins() || rhs[0].pin as usize == jedec.chip.num_pins() / 2) {
        if rhs[0].neg != 0 {
            // /VCC and /GND are not allowed
            return Err(eqn.line_num * 0x10000 + 25);
        }

        if rhs[0].pin as usize == jedec.chip.num_pins() / 2 {
            jedec.clear_row(start_row, row_offset);
        }
    } else {
        for i in 0..rhs.len() {
            let pin_num = rhs[i].pin;

            if pin_num as usize == jedec.chip.num_pins() || pin_num as usize == jedec.chip.num_pins() / 2 {
                return Err(eqn.line_num * 0x10000 + 28);
            }

            if ops[i] == 43 || ops[i] == 35 {
                // If an OR, go to next row.
                row_offset += 1;

                if row_offset == max_row {
                    // too many ORs?
                    return Err(eqn.line_num * 0x10000 + 30);
                }
            }

            // Set ANDs.
            if let Err(i) =  set_and(jedec, start_row + row_offset, pin_num as usize, rhs[i].neg != 0) {
                return Err(eqn.line_num * 0x10000 + i);
            }
        }
    }

    // Then zero the rest...
    row_offset += 1;
    jedec.clear_rows(start_row, row_offset, max_row);

    Ok(())
}

pub fn do_it_all(
    jedec: &mut Jedec,
    blueprint: &mut Blueprint,
    eqns: &[Equation],
    file: &str,
) -> Result<(), i32> {

    // Convert equations into data on the OLMCs.
    for eqn in eqns.iter() {
        if let Err(err) = blueprint.add_equation(eqn, jedec) {
            return Err(eqn.line_num * 0x10000 + err); // TODO: Ick.
        }
    }

    // Complete second pass from in-memory structure.
    println!("Assembler Phase 2 for \"{}\"", file);

    let mode = match olmc::analyse_mode(jedec, &mut blueprint.olmcs) {
        Some(Mode::Mode1) => 1,
        Some(Mode::Mode2) => 2,
        Some(Mode::Mode3) => 3,
        None => 0,
    };

    println!("GAL {}; Operation mode {}; Security fuse {}",
             &jedec.chip.name()[3..],
             mode,
             "off"); // TODO cfg->JedecSecBit ? "on" : "off");


    // NB: Length of num_olmcs may be incorrect because that includes AR, SP, etc.
    for i in 0..jedec.chip.num_olmcs() {
        if let Some(eqn) = blueprint.olmcs[i].output {
            add_equation(jedec, &blueprint.olmcs, &eqn)?;
        }
        if let Some(eqn) = blueprint.olmcs[i].arst {
            add_equation(jedec, &blueprint.olmcs, &eqn)?;
        }
        if let Some(eqn) = blueprint.olmcs[i].aprst {
            add_equation(jedec, &blueprint.olmcs, &eqn)?;
        }
        if let Some(eqn) = blueprint.olmcs[i].clock {
            add_equation(jedec, &blueprint.olmcs, &eqn)?;
        }
        if let olmc::Tri::Some(eqn) = blueprint.olmcs[i].tri_con {
            add_equation(jedec, &blueprint.olmcs, &eqn)?;
        }

        if blueprint.olmcs[i].pin_type == PinType::UNDRIVEN {
            jedec.clear_olmc(i);
        }

        if jedec.chip == Chip::GAL20RA10 {
            if blueprint.olmcs[i].pin_type != PinType::UNDRIVEN {
                if blueprint.olmcs[i].pin_type == PinType::REGOUT && blueprint.olmcs[i].clock.is_none() {
                    // return Err(format?("missing clock definition (.CLK) of registered output on pin {}", n + 14));
                    return Err(41); // FIXME i + 14);
                }

                if blueprint.olmcs[i].clock.is_none() {
                    let start_row = jedec.chip.start_row_for_olmc(i);
                    jedec.clear_row(start_row, 1);
                }

                if blueprint.olmcs[i].pin_type == PinType::REGOUT {
                    if blueprint.olmcs[i].arst.is_none() {
                        let start_row = jedec.chip.start_row_for_olmc(i);
                        jedec.clear_row(start_row, 2);
                    }

                    if blueprint.olmcs[i].aprst.is_none() {
                        let start_row = jedec.chip.start_row_for_olmc(i);
                        jedec.clear_row(start_row, 3);
                    }
                }
            }
        }
    }

    // Special cases
    if jedec.chip == Chip::GAL22V10 {
        if let Some(eqn) = blueprint.olmcs[10].output {
            add_equation(jedec, &blueprint.olmcs, &eqn)?;
        }

        if blueprint.olmcs[10].pin_type == PinType::UNDRIVEN    /* set row of AR equal 0 */ {
            jedec.clear_olmc(10);
        }

        if let Some(eqn) = blueprint.olmcs[11].output {
            add_equation(jedec, &blueprint.olmcs, &eqn)?;
        }

        if blueprint.olmcs[11].pin_type == PinType::UNDRIVEN    /* set row of SP equal 0 */ {
            jedec.clear_olmc(11);
        }
    }

    Ok(())
}

pub fn do_stuff(
    gal_type: Chip,
    sig: &[u8],
    eqns: &[Equation],
    file: &str,
    pin_names: &[&str],
    config: &::jedec_writer::Config,
) -> Result<(), i32> {
    let mut jedec = Jedec::new(gal_type);

    let mut blueprint = Blueprint::new();

    // Set signature.
    for x in jedec.sig.iter_mut() {
        *x = false;
    }

    // Signature has space for 8 bytes.
    for i in 0..usize::min(sig.len(), 8) {
        let c = sig[i];
        for j in 0..8 {
            jedec.sig[i * 8 + j] = (c << j) & 0x80 != 0;
        }
    }

    do_it_all(&mut jedec, &mut blueprint, eqns, file)?;

    writer::write_files(file, config, pin_names, &blueprint.olmcs, &jedec).unwrap();

    Ok(())
}
