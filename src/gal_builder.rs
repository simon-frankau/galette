use blueprint::Blueprint;
use chips::Chip;
use jedec;
use jedec::Bounds;
use jedec::Jedec;
use jedec::Mode;
use olmc;
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

pub const SUFFIX_NON: i32 =              0;	/* possible suffixes */
pub const SUFFIX_T: i32 =                1;
pub const SUFFIX_R: i32 =                2;
pub const SUFFIX_E: i32 =                3;
pub const SUFFIX_CLK: i32 =              4;
pub const SUFFIX_APRST: i32 =            5;
pub const SUFFIX_ARST: i32 =             6;

// Adjust the bounds for the main term of there's a tristate enable
// term in the first row.
pub fn tristate_adjust(jedec: &Jedec,pin_type: PinType, bounds: &Bounds) -> Bounds {
    match jedec.chip {
        Chip::GAL16V8 | Chip::GAL20V8 => {
            if jedec.get_mode() != Mode::Mode1 && pin_type != PinType::REGOUT {
                Bounds { row_offset: 1, ..*bounds }
            } else {
                *bounds
            }
        }
        Chip::GAL22V10 => Bounds { row_offset: 1, ..*bounds },
        Chip::GAL20RA10 => panic!("Nope!"),
    }
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

    match jedec.chip {
        Chip::GAL16V8 | Chip::GAL20V8 => build_galxv8(jedec, blueprint)?,
        Chip::GAL22V10 => build_gal22v10(jedec, blueprint)?,
        Chip::GAL20RA10 => build_gal20ra10(jedec, blueprint)?,
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

fn build_galxvx(jedec: &mut Jedec, blueprint: &mut Blueprint) -> Result<(), i32> {
    // NB: Length of num_olmcs may be incorrect because that includes AR, SP, etc.
    for i in 0..jedec.chip.num_olmcs() {
        let bounds = jedec.chip.get_bounds(i);

        match &blueprint.olmcs[i].output {
            Some(term) => {
                let bounds = tristate_adjust(jedec, blueprint.olmcs[i].pin_type, &bounds);
                jedec.add_term(&term, &bounds)?;
            }
            None => jedec.add_term(&jedec::false_term(0), &bounds)?,
        }

        if let Some(term) = &blueprint.olmcs[i].tri_con {
            jedec.add_term(&term, &Bounds { row_offset: 0, max_row: 1, ..bounds })?;
        }
    }

    Ok(())
}

fn build_galxv8(jedec: &mut Jedec, blueprint: &mut Blueprint) -> Result<(), i32> {
    build_galxvx(jedec, blueprint)?;

    Ok(())
}

fn build_gal22v10(jedec: &mut Jedec, blueprint: &mut Blueprint) -> Result<(), i32> {
    build_galxvx(jedec, blueprint)?;

    // AR
    let ar_bounds = jedec.chip.get_bounds(10);
    jedec.add_term_opt(&blueprint.olmcs[10].output, &ar_bounds)?;

    // SP
    let sp_bounds = jedec.chip.get_bounds(11);
    jedec.add_term_opt(&blueprint.olmcs[11].output, &sp_bounds)?;

    Ok(())
}

fn build_gal20ra10(jedec: &mut Jedec, blueprint: &mut Blueprint) -> Result<(), i32> {
    // NB: Length of num_olmcs may be incorrect because that includes AR, SP, etc.
    for i in 0..jedec.chip.num_olmcs() {
        let bounds = jedec.chip.get_bounds(i);

        match &blueprint.olmcs[i].output {
            Some(term) => {
                jedec.add_term(&term, &Bounds { row_offset: 4, .. bounds })?;
            }
            None => jedec.add_term(&jedec::false_term(0), &bounds)?,
        }

        if let Some(term) = &blueprint.olmcs[i].tri_con {
            jedec.add_term(&term, &Bounds { row_offset: 0, max_row: 1, .. bounds })?;
        }

        if blueprint.olmcs[i].pin_type != PinType::UNDRIVEN {
            if blueprint.olmcs[i].pin_type == PinType::REGOUT && blueprint.olmcs[i].clock.is_none() {
                // return Err(format?("missing clock definition (.CLK) of registered output on pin {}", n + 14));
                return Err(41); // FIXME i + 14);
            }

            let clock_bounds = Bounds { row_offset: 1, max_row: 2, .. bounds };
            jedec.add_term_opt(&blueprint.olmcs[i].clock, &clock_bounds)?;

            if blueprint.olmcs[i].pin_type == PinType::REGOUT {
                let arst_bounds = Bounds { row_offset: 2, max_row: 3, .. bounds };
                jedec.add_term_opt(&blueprint.olmcs[i].arst, &arst_bounds)?;

                let aprst_bounds = Bounds { row_offset: 3, max_row: 4, .. bounds };
                jedec.add_term_opt(&blueprint.olmcs[i].aprst, &aprst_bounds)?;
            }
        }
    }

    Ok(())
}
