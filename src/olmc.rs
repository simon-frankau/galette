// TODO: This logic should be rolled into the gal_builder.

use blueprint::OLMC;
use blueprint::Active;
use blueprint::PinMode;
use chips::Chip;
use gal;
use gal::Mode;

////////////////////////////////////////////////////////////////////////
// Analyse OLMCs

// Get the mode for GAL16V8 and GAL20V8, set the flags appropriately
pub fn analyse_mode_v8(gal: &mut gal::GAL, olmcs: &[OLMC]) -> Mode {
    let mode = get_mode_v8(olmcs);
    gal.set_mode(mode);
    return mode;
}

pub fn get_mode_v8(olmcs: &[OLMC]) -> Mode {
    // If there's a registered pin, it's mode 3.
    for n in 0..8 {
        if let Some((PinMode::Registered, _)) = olmcs[n].output  {
            return Mode::Mode3;
        }
    }
    // If there's a tristate, it's mode 2.
    for n in 0..8 {
        if let Some((PinMode::Tristate, _)) = olmcs[n].output {
            return Mode::Mode2;
        }
    }
    // If we can't use mode 1, use mode 2.
    for n in 0..8 {
        // Some OLMCs cannot be configured as pure inputs in Mode 1.
        if olmcs[n].feedback && olmcs[n].output.is_none() {
            if n == 3 || n == 4 {
                return Mode::Mode2;
            }
        }
        // OLMC pins cannot be used as combinatorial feedback in Mode 1.
        if olmcs[n].feedback && olmcs[n].output.is_some() {
            return Mode::Mode2;
        }
    }
    // If there is still no mode defined, use mode 1.
    return Mode::Mode1;
}

pub fn analyse_mode(gal: &mut gal::GAL, olmcs: &mut [OLMC]) -> Option<gal::Mode> {
    match gal.chip {
        Chip::GAL16V8 | Chip::GAL20V8 => {
            let mode = analyse_mode_v8(gal, olmcs);

            if mode != Mode::Mode1 {
                // Convert combinatorial expressions into tristate ones,
                // adding a trivial (always true) enable term.
                for n in 0..8 {
                    if let Some((ref mut pin_mode, ref term)) = olmcs[n].output {
                        if *pin_mode == PinMode::Combinatorial {
                            *pin_mode = PinMode::Tristate;
                            olmcs[n].tri_con = Some(gal::true_term(term.line_num));
                        }
                    }
                }
            }

            // SYN and AC0 already defined.

            for n in 0..64 {
                gal.pt[n] = true;
            }

            for n in 0..8 {
                if match olmcs[n].output {
                    None => olmcs[n].feedback,
                    Some((PinMode::Tristate, _)) => true,
                    _ => false,
                } {
                    gal.ac1[7 - n] = true;
                }
            }

            for n in 0..8 {
                if olmcs[n].output.is_some() && olmcs[n].active == Active::High {
                    gal.xor[7 - n] = true;
                }
            }

            return Some(mode);
        }

        Chip::GAL22V10 => {
            for n in 0..10 {
                // Make combinatorial terms into tristates.
                if let Some((ref mut pin_mode, _)) = olmcs[n].output {
                    if *pin_mode == PinMode::Combinatorial {
                        *pin_mode = PinMode::Tristate;
                    }
                }

                if olmcs[n].output.is_some() && olmcs[n].active == Active::High {
                    gal.xor[9 - n] = true;
                }

                if match olmcs[n].output {
                    None => olmcs[n].feedback,
                    Some((PinMode::Tristate, _)) => true,
                    _ => false,
                } {
                    gal.s1[9 - n] = true;
                }
            }
        }

        Chip::GAL20RA10 => {
            for n in 0..10 {
                // Make combinatorial terms into tristates.
                if let Some((ref mut pin_mode, _)) = olmcs[n].output {
                    if *pin_mode == PinMode::Combinatorial {
                        *pin_mode = PinMode::Tristate;
                    }
                }

                if olmcs[n].output.is_some() && olmcs[n].active == Active::High {
                    gal.xor[9 - n] = true;
                }
            }
        }
    }

    None
}
