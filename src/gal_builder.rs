use chips::Chip;
use jedec::Jedec;
use jedec::Mode;
use olmc;
use olmc::OLMC;
use writer;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Pin {
    neg: i8,
    pin: i8,
}

// Config use on the C side.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

pub fn mark_input(
    jedec: &Jedec,
    olmcs: &mut [OLMC],
    act_pin: &Pin,
) {
    if let Some(n) = jedec.chip.pin_to_olmc(act_pin.pin as usize) {
        if olmcs[n].pin_type == olmc::NOTUSED {
            olmcs[n].pin_type = olmc::INPUT;
        }
        olmcs[n].feedback = 1;
    }
}

// Pin types:
// NOT USED
//  -> INPUT if used as input
//

pub fn register_output(
    jedec: &Jedec,
    olmcs: &mut [OLMC],
    act_pin: &Pin,
    suffix: i32
) -> Result<(), i32> {
    let olmc = match jedec.chip.pin_to_olmc(act_pin.pin as usize) {
        None => return Err(15),
        Some(olmc) => olmc,
    };

    match suffix {
        SUFFIX_R | SUFFIX_T | SUFFIX_NON =>
            register_output_base(jedec, &mut olmcs[olmc], act_pin, suffix, olmc >= 10),
        SUFFIX_E =>
            register_output_enable(jedec, &mut olmcs[olmc], act_pin),
        SUFFIX_CLK =>
            register_output_clock(&mut olmcs[olmc], act_pin),
        SUFFIX_ARST =>
            register_output_arst(&mut olmcs[olmc], act_pin),
        SUFFIX_APRST =>
            register_output_aprst(&mut olmcs[olmc], act_pin),
        _ =>
            panic!("Nope"),
    }
}

fn register_output_base(
    jedec: &Jedec,
    olmc: &mut OLMC,
    act_pin: &Pin,
    suffix: i32,
    is_arsp: bool, // TODO: Hack for the error message?
) -> Result<(), i32> {
    if olmc.pin_type == 0 || olmc.pin_type == olmc::INPUT {
        if act_pin.neg != 0 {
            olmc.active = olmc::ACTIVE_LOW;
        } else {
            olmc.active = olmc::ACTIVE_HIGH;
        }

        if suffix == SUFFIX_T {
            olmc.pin_type = olmc::TRIOUT;
        }

        if suffix == SUFFIX_R {
            olmc.pin_type = olmc::REGOUT;
        }

        if suffix == SUFFIX_NON {
            olmc.pin_type = olmc::COM_TRI_OUT;
        }
    } else {
        if jedec.chip == Chip::GAL22V10 && is_arsp {
            return Err(40);
        } else {
            return Err(16);
        }
    }

    Ok(())
}

fn register_output_enable(
    jedec: &Jedec,
    olmc: &mut OLMC,
    act_pin: &Pin,
) -> Result<(), i32> {
    if act_pin.neg != 0 {
        return Err(19);
    }

    if olmc.tri_con != 0 {
        return Err(22);
    }

    olmc.tri_con = 1;

    if olmc.pin_type == 0 || olmc.pin_type == olmc::INPUT {
        return Err(17);
    }

    if olmc.pin_type == olmc::REGOUT && (jedec.chip == Chip::GAL16V8 || jedec.chip == Chip::GAL20V8) {
        return Err(23);
    }

    if olmc.pin_type == olmc::COM_TRI_OUT {
        return Err(24);
    }

    Ok(())
}

fn register_output_clock(
    olmc: &mut OLMC,
    act_pin: &Pin,
) -> Result<(), i32> {
    if act_pin.neg != 0 {
        return Err(19);
    }

    if olmc.pin_type == olmc::NOTUSED {
        return Err(42);
    }

    if olmc.clock != 0 {
        return Err(45);
    }

    olmc.clock = 1;
    if olmc.pin_type != olmc::REGOUT {
        return Err(48);
    }

    Ok(())
}

fn register_output_arst(
    olmc: &mut OLMC,
    act_pin: &Pin,
) -> Result<(), i32> {
    if act_pin.neg != 0 {
        return Err(19);
    }

    if olmc.pin_type == olmc::NOTUSED {
        return Err(43);
    }

    if olmc.arst != 0 {
        return Err(46);
    }

    olmc.arst = 1;
    if olmc.pin_type != olmc::REGOUT {
        return Err(48);
    }

    Ok(())
}

fn register_output_aprst(
    olmc: &mut OLMC,
    act_pin: &Pin,
) -> Result<(), i32> {
    if act_pin.neg != 0 {
        return Err(19);
    }

    if olmc.pin_type == olmc::NOTUSED {
        return Err(44);
    }

    if olmc.aprst != 0 {
        return Err(47);
    }

    olmc.aprst = 1;
    if olmc.pin_type != olmc::REGOUT {
        return Err(48);
    }

    Ok(())
}

pub fn do_it_all(
    jedec: &mut Jedec,
    olmcs: &mut [OLMC],
    eqns: &[Equation],
    file: &str,
) -> Result<(), i32> {
    // Collect marks.
    for eqn in eqns.iter() {
        let olmc = match jedec.chip.pin_to_olmc(eqn.lhs.pin as usize) {
            None => return Err(15),
            Some(olmc) => olmc,
        };
        olmcs[olmc].eqns.push(*eqn);

        if let Err(err) = register_output(jedec, olmcs, &eqn.lhs, eqn.suffix) {
            return Err(eqn.line_num * 0x10000 + err); // TODO: Ick.
        }

        let rhs = unsafe { std::slice::from_raw_parts(eqn.rhs, eqn.num_rhs as usize) };

        for input in rhs.iter() {
            mark_input(jedec, olmcs, input);
        }
    }

    // Complete second pass from in-memory structure.
    println!("Assembler Phase 2 for \"{}\"", file);

    let mode = match olmc::analyse_mode(jedec, olmcs) {
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
        for eqn in olmcs[i].eqns.iter() {
            let rhs = unsafe { std::slice::from_raw_parts(eqn.rhs, eqn.num_rhs as usize) };
            let ops = unsafe { std::slice::from_raw_parts(eqn.ops, eqn.num_rhs as usize) };

            if let Err(err) = add_equation(jedec, olmcs, eqn.line_num, &eqn.lhs, eqn.suffix, rhs, ops) {
                return Err(eqn.line_num * 0x10000 + err);
            }
        }

        if olmcs[i].pin_type == olmc::NOTUSED || olmcs[i].pin_type == olmc::INPUT {
            jedec.clear_olmc(i);
        }

        if jedec.chip == Chip::GAL20RA10 {
            if olmcs[i].pin_type != olmc::NOTUSED {
                if olmcs[i].pin_type == olmc::REGOUT && olmcs[i].clock == 0 {
                    // return Err(format?("missing clock definition (.CLK) of registered output on pin {}", n + 14));
                    return Err(41); // FIXME i + 14);
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

    // Special cases
    if jedec.chip == Chip::GAL22V10 {
        {
            for eqn in olmcs[10].eqns.iter() {
                let rhs = unsafe { std::slice::from_raw_parts(eqn.rhs, eqn.num_rhs as usize) };
                let ops = unsafe { std::slice::from_raw_parts(eqn.ops, eqn.num_rhs as usize) };

                if let Err(err) = add_equation(jedec, olmcs, eqn.line_num, &eqn.lhs, eqn.suffix, rhs, ops) {
                    return Err(eqn.line_num * 0x10000 + err);
                }
            }
        }

        if olmcs[10].pin_type == 0    /* set row of AR equal 0 */ {
            jedec.clear_olmc(10);
        }

        {
            for eqn in olmcs[11].eqns.iter() {
                let rhs = unsafe { std::slice::from_raw_parts(eqn.rhs, eqn.num_rhs as usize) };
                let ops = unsafe { std::slice::from_raw_parts(eqn.ops, eqn.num_rhs as usize) };

                if let Err(err) = add_equation(jedec, olmcs, eqn.line_num, &eqn.lhs, eqn.suffix, rhs, ops) {
                    return Err(eqn.line_num * 0x10000 + err);
                }
            }
        }

        if olmcs[11].pin_type == 0    /* set row of SP equal 0 */ {
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

    // Set up OLMCs.
    let mut olmcs = vec!(OLMC {
        active: 0,
        pin_type: 0,
        tri_con: 0,
        clock: 0,
        arst: 0,
        aprst: 0,
        feedback: 0,
        eqns: Vec::new(),
     };12);

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

    do_it_all(&mut jedec, &mut olmcs, eqns, file)?;

    let olmc_pin_types = olmcs.iter().map(|x| x.pin_type as i32).collect::<Vec<i32>>();

    writer::write_files(file, config, pin_names, &olmc_pin_types, &jedec).unwrap();

    Ok(())
}
