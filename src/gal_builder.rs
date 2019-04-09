use blueprint::Blueprint;
use chips::Chip;
use errors::Error;
use errors::ErrorCode;
use gal;
use gal::Bounds;
use gal::GAL;
use gal::Mode;
use olmc;
use olmc::PinType;
use writer;

pub use gal::Pin;

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
pub fn tristate_adjust(gal: &GAL,pin_type: PinType, bounds: &Bounds) -> Bounds {
    match gal.chip {
        Chip::GAL16V8 | Chip::GAL20V8 => {
            if gal.get_mode() != Mode::Mode1 && pin_type != PinType::REGOUT {
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
    gal: &mut GAL,
    blueprint: &mut Blueprint,
    eqns: &[Equation],
    file: &str,
) -> Result<(), Error> {

    // Convert equations into data on the OLMCs.
    for eqn in eqns.iter() {
        if let Err(err) = blueprint.add_equation(eqn, gal) {
            return Err(Error { code: err, line: eqn.line_num });
        }
    }

    // Complete second pass from in-memory structure.
    println!("Assembler Phase 2 for \"{}\"", file);

    let mode = match olmc::analyse_mode(gal, &mut blueprint.olmcs) {
        Some(Mode::Mode1) => 1,
        Some(Mode::Mode2) => 2,
        Some(Mode::Mode3) => 3,
        None => 0,
    };

    println!("GAL {}; Operation mode {}; Security fuse {}",
             &gal.chip.name()[3..],
             mode,
             "off"); // TODO cfg->JedecSecBit ? "on" : "off");

    match gal.chip {
        Chip::GAL16V8 | Chip::GAL20V8 => build_galxv8(gal, blueprint)?,
        Chip::GAL22V10 => build_gal22v10(gal, blueprint)?,
        Chip::GAL20RA10 => build_gal20ra10(gal, blueprint)?,
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
) -> Result<(), Error> {
    let mut gal = GAL::new(gal_type);

    let mut blueprint = Blueprint::new(gal_type);

    // Set signature.
    for x in gal.sig.iter_mut() {
        *x = false;
    }

    // Signature has space for 8 bytes.
    for i in 0..usize::min(sig.len(), 8) {
        let c = sig[i];
        for j in 0..8 {
            gal.sig[i * 8 + j] = (c << j) & 0x80 != 0;
        }
    }

    do_it_all(&mut gal, &mut blueprint, eqns, file)?;

    writer::write_files(file, config, pin_names, &blueprint.olmcs, &gal).unwrap();

    Ok(())
}

fn build_galxvx(gal: &mut GAL, blueprint: &mut Blueprint) -> Result<(), Error> {
    for i in 0..blueprint.olmcs.len() {
        let bounds = gal.chip.get_bounds(i);

        match &blueprint.olmcs[i].output {
            Some(term) => {
                let bounds = tristate_adjust(gal, blueprint.olmcs[i].pin_type, &bounds);
                gal.add_term(&term, &bounds)?;
            }
            None => gal.add_term(&gal::false_term(0), &bounds)?,
        }

        if let Some(term) = &blueprint.olmcs[i].tri_con {
            gal.add_term(&term, &Bounds { row_offset: 0, max_row: 1, ..bounds })?;
        }
    }

    Ok(())
}

fn build_galxv8(gal: &mut GAL, blueprint: &mut Blueprint) -> Result<(), Error> {
    build_galxvx(gal, blueprint)?;

    Ok(())
}

fn build_gal22v10(gal: &mut GAL, blueprint: &mut Blueprint) -> Result<(), Error> {
    build_galxvx(gal, blueprint)?;

    // AR
    let ar_bounds = Bounds { start_row: 0, max_row: 1, row_offset: 0 };
    gal.add_term_opt(&blueprint.ar, &ar_bounds)?;

    // SP
    let sp_bounds = Bounds { start_row: 131, max_row: 1, row_offset: 0 };
    gal.add_term_opt(&blueprint.sp, &sp_bounds)?;

    Ok(())
}

fn build_gal20ra10(gal: &mut GAL, blueprint: &mut Blueprint) -> Result<(), Error> {
    for i in 0..blueprint.olmcs.len() {
        let bounds = gal.chip.get_bounds(i);

        match &blueprint.olmcs[i].output {
            Some(term) => {
                gal.add_term(&term, &Bounds { row_offset: 4, .. bounds })?;
            }
            None => gal.add_term(&gal::false_term(0), &bounds)?,
        }

        if let Some(term) = &blueprint.olmcs[i].tri_con {
            gal.add_term(&term, &Bounds { row_offset: 0, max_row: 1, .. bounds })?;
        }

        if blueprint.olmcs[i].pin_type != PinType::UNDRIVEN {
            if blueprint.olmcs[i].pin_type == PinType::REGOUT && blueprint.olmcs[i].clock.is_none() {
                // return Err(format?("missing clock definition (.CLK) of registered output on pin {}", n + 14));
                return Err(Error { code: ErrorCode::NO_CLK, line: 0 }); // FIXME i + 14);
            }

            let clock_bounds = Bounds { row_offset: 1, max_row: 2, .. bounds };
            gal.add_term_opt(&blueprint.olmcs[i].clock, &clock_bounds)?;

            if blueprint.olmcs[i].pin_type == PinType::REGOUT {
                let arst_bounds = Bounds { row_offset: 2, max_row: 3, .. bounds };
                gal.add_term_opt(&blueprint.olmcs[i].arst, &arst_bounds)?;

                let aprst_bounds = Bounds { row_offset: 3, max_row: 4, .. bounds };
                gal.add_term_opt(&blueprint.olmcs[i].aprst, &aprst_bounds)?;
            }
        }
    }

    Ok(())
}
