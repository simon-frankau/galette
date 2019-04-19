use chips::Chip;
use errors::ErrorCode;
use gal_builder::Pin;
use gal;
use gal::GAL;
use gal::Mode;
use gal::Term;
use parser::Suffix;

#[derive(Clone, Debug, PartialEq)]
pub enum PinMode {
    ComOut,
    TriOut,
    RegOut,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Active {
    LOW,
    HIGH
}

#[derive(Clone, Debug)]
pub struct OLMC {
    pub active: Active,
    pub output: Option<(PinMode, gal::Term)>,
    pub tri_con: Option<gal::Term>,
    pub clock: Option<gal::Term>,
    pub arst: Option<gal::Term>,
    pub aprst: Option<gal::Term>,
    pub feedback: bool,
}

////////////////////////////////////////////////////////////////////////
// Build OLMCs

impl OLMC {
    pub fn set_base(
        &mut self,
        act_pin: &Pin,
        term: Term,
        suffix: Suffix,
    ) -> Result<(), ErrorCode> {
        if self.output.is_some() {
            // Previously defined, so error out.
            return Err(ErrorCode::RepeatedOutput);
        }

        self.output = Some((match suffix {
            Suffix::T => PinMode::TriOut,
            Suffix::R => PinMode::RegOut,
            Suffix::None => PinMode::ComOut,
            _ => panic!("Nope!"),
        }, term));

        self.active = if act_pin.neg {
            Active::LOW
        } else {
            Active::HIGH
        };

        Ok(())
    }

    pub fn set_enable(
        &mut self,
        gal: &GAL,
        act_pin: &Pin,
        term: Term,
    ) -> Result<(), ErrorCode> {
        if act_pin.neg {
            return Err(ErrorCode::InvertedControl);
        }

        if self.tri_con != None {
            return Err(ErrorCode::RepeatedTristate);
        }

        self.tri_con = Some(term);

        match self.output {
            None => return Err(ErrorCode::PrematureENABLE),
            Some((PinMode::RegOut, _)) => {
                if gal.chip == Chip::GAL16V8 || gal.chip == Chip::GAL20V8 {
                    return Err(ErrorCode::TristateReg);
                }
            }
            Some((PinMode::ComOut, _)) => return Err(ErrorCode::UnmatchedTristate),
            _ => {}
        }

        Ok(())
    }

    pub fn set_clock(
        &mut self,
        act_pin: &Pin,
        term: Term,
    ) -> Result<(), ErrorCode> {
        if act_pin.neg {
            return Err(ErrorCode::InvertedControl);
        }

        match self.output {
            None => return Err(ErrorCode::PrematureCLK),
            Some((PinMode::RegOut, _)) => {}
            _ => return Err(ErrorCode::InvalidControl),
        }

        if self.clock.is_some() {
            return Err(ErrorCode::RepeatedCLK);
        }
        self.clock = Some(term);

        Ok(())
    }

    pub fn set_arst(
        &mut self,
        act_pin: &Pin,
        term: Term
    ) -> Result<(), ErrorCode> {
        if act_pin.neg {
            return Err(ErrorCode::InvertedControl);
        }

        match self.output {
            None => return Err(ErrorCode::PrematureARST),
            Some((PinMode::RegOut, _)) => {}
            _ => return Err(ErrorCode::InvalidControl),
        };

        if self.arst.is_some() {
            return Err(ErrorCode::RepeatedARST);
        }
        self.arst = Some(term);

        Ok(())
    }

    pub fn set_aprst(
        &mut self,
        act_pin: &Pin,
        term: Term,
    ) -> Result<(), ErrorCode> {
        if act_pin.neg {
            return Err(ErrorCode::InvertedControl);
        }

        match self.output {
            None => return Err(ErrorCode::PrematureAPRST),
            Some((PinMode::RegOut, _)) => {}
            _ => return Err(ErrorCode::InvalidControl),
        }

        if self.aprst.is_some() {
            return Err(ErrorCode::RepeatedAPRST);
        }
        self.aprst = Some(term);

        Ok(())
    }
}

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
        if let Some((PinMode::RegOut, _)) = olmcs[n].output  {
            return Mode::Mode3;
        }
    }
    // If there's a tristate, it's mode 2.
    for n in 0..8 {
        if let Some((PinMode::TriOut, _)) = olmcs[n].output {
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
                        if *pin_mode == PinMode::ComOut {
                            *pin_mode = PinMode::TriOut;
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
                    Some((PinMode::TriOut, _)) => true,
                    _ => false,
                } {
                    gal.ac1[7 - n] = true;
                }
            }

            for n in 0..8 {
                if olmcs[n].output.is_some() && olmcs[n].active == Active::HIGH {
                    gal.xor[7 - n] = true;
                }
            }

            return Some(mode);
        }

        Chip::GAL22V10 => {
            for n in 0..10 {
                // Make combinatorial terms into tristates.
                if let Some((ref mut pin_mode, _)) = olmcs[n].output {
                    if *pin_mode == PinMode::ComOut {
                        *pin_mode = PinMode::TriOut;
                    }
                }

                if olmcs[n].output.is_some() && olmcs[n].active == Active::HIGH {
                    gal.xor[9 - n] = true;
                }

                if match olmcs[n].output {
                    None => olmcs[n].feedback,
                    Some((PinMode::TriOut, _)) => true,
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
                    if *pin_mode == PinMode::ComOut {
                        *pin_mode = PinMode::TriOut;
                    }
                }

                if olmcs[n].output.is_some() && olmcs[n].active == Active::HIGH {
                    gal.xor[9 - n] = true;
                }
            }
        }
    }

    None
}
