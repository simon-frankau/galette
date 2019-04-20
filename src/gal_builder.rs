use blueprint::Blueprint;
use blueprint::PinMode;
use chips::Chip;
use errors::at_line;
use errors::Error;
use errors::ErrorCode;
use gal;
use gal::Bounds;
use gal::GAL;
use gal::Mode;
use olmc;


pub fn build(blueprint: &mut Blueprint) -> Result<GAL, Error> {
    let mut gal = GAL::new(blueprint.chip);

    // Set signature.
    for x in gal.sig.iter_mut() {
        *x = false;
    }

    // Signature has space for 8 bytes.
    for i in 0..usize::min(blueprint.sig.len(), 8) {
        let c = blueprint.sig[i];
        for j in 0..8 {
            gal.sig[i * 8 + j] = (c << j) & 0x80 != 0;
        }
    }

    let mode = match olmc::analyse_mode(&mut gal, &mut blueprint.olmcs) {
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
        Chip::GAL16V8 | Chip::GAL20V8 => build_galxv8(&mut gal, blueprint)?,
        Chip::GAL22V10 => build_gal22v10(&mut gal, blueprint)?,
        Chip::GAL20RA10 => build_gal20ra10(&mut gal, blueprint)?,
    }

    Ok(gal)
}

// Adjust the bounds for the main term of there's a tristate enable
// term in the first row.
pub fn tristate_adjust(gal: &GAL, output: &Option<(PinMode, gal::Term)>, bounds: &Bounds) -> Bounds {
    match gal.chip {
        Chip::GAL16V8 | Chip::GAL20V8 => {
            let reg_out = if let Some((PinMode::Registered, _)) = output { true } else { false };
            if gal.get_mode() != Mode::Mode1 && !reg_out {
                Bounds { row_offset: 1, ..*bounds }
            } else {
                *bounds
            }
        }
        Chip::GAL22V10 => Bounds { row_offset: 1, ..*bounds },
        Chip::GAL20RA10 => panic!("Nope!"),
    }
}

// Check that you're not trying to use 20ra10-specific features
fn check_gal20ra10(blueprint: &mut Blueprint) -> Result<(), Error> {
    for olmc in blueprint.olmcs.iter() {
        if let Some(term) = &olmc.clock {
            return at_line(term.line_num, Err(ErrorCode::DisallowedCLK));
        }
        if let Some(term) = &olmc.arst {
            return at_line(term.line_num, Err(ErrorCode::DisallowedARST));
        }
        if let Some(term) = &olmc.aprst {
            return at_line(term.line_num, Err(ErrorCode::DisallowedAPRST));
        }
    }
    Ok(())
}

fn build_galxvx(gal: &mut GAL, blueprint: &mut Blueprint) -> Result<(), Error> {
    for i in 0..blueprint.olmcs.len() {
        let bounds = gal.chip.get_bounds(i);

        match &blueprint.olmcs[i].output {
            Some((_, term)) => {
                let bounds = tristate_adjust(gal, &blueprint.olmcs[i].output, &bounds);
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
    check_gal20ra10(blueprint)?;
    build_galxvx(gal, blueprint)?;
    Ok(())
}

fn build_gal22v10(gal: &mut GAL, blueprint: &mut Blueprint) -> Result<(), Error> {
    check_gal20ra10(blueprint)?;
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
        let olmc = &blueprint.olmcs[i];

        match &olmc.output {
            Some((_, term)) => {
                gal.add_term(&term, &Bounds { row_offset: 4, .. bounds })?;
            }
            None => gal.add_term(&gal::false_term(0), &bounds)?,
        }

        if let Some(term) = &olmc.tri_con {
            gal.add_term(&term, &Bounds { row_offset: 0, max_row: 1, .. bounds })?;
        }

        if olmc.output.is_some() {
            if let Some((PinMode::Registered, ref term)) = olmc.output {
                let arst_bounds = Bounds { row_offset: 2, max_row: 3, .. bounds };
                gal.add_term_opt(&olmc.arst, &arst_bounds)?;

                let aprst_bounds = Bounds { row_offset: 3, max_row: 4, .. bounds };
                gal.add_term_opt(&olmc.aprst, &aprst_bounds)?;

                if olmc.clock.is_none() {
                    return at_line(term.line_num, Err(ErrorCode::NoCLK));
                }
            }

            let clock_bounds = Bounds { row_offset: 1, max_row: 2, .. bounds };
            gal.add_term_opt(&olmc.clock, &clock_bounds)?;
        }
    }

    Ok(())
}
