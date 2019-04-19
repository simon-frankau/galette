use chips::Chip;
use errors::ErrorCode;
use gal_builder::Pin;
use gal;
use gal::GAL;
use gal::Mode;
use gal::Term;
use parser::Suffix;

#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    Undriven,
    ComOut(gal::Term),
    TriOut(gal::Term),
    RegOut(gal::Term),
    ComTriOut(gal::Term),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Active {
    LOW,
    HIGH
}

#[derive(Clone, Debug)]
pub struct OLMC {
    pub active: Active,
    pub output: Output,
    pub tri_con: Option<gal::Term>,
    pub clock: Option<gal::Term>,
    pub arst: Option<gal::Term>,
    pub aprst: Option<gal::Term>,
    pub feedback: bool,
}

////////////////////////////////////////////////////////////////////////
// Build OLMCs

// Pin types:
// NOT USED (Can also be only used as input)
//  -> TriOut - tristate
//  -> RegOut - registered
//  -> ComTriOut - combinatorial, might be tristated.
//     analysed to:
//     -> ComOut
//     -> TriOut

impl OLMC {
    pub fn set_base(
        &mut self,
        act_pin: &Pin,
        term: Term,
        suffix: Suffix,
    ) -> Result<(), ErrorCode> {
        if self.output != Output::Undriven {
            // Previously defined, so error out.
            return Err(ErrorCode::RepeatedOutput);
        }

        self.output = match suffix {
            Suffix::T => Output::TriOut(term),
            Suffix::R => Output::RegOut(term),
            Suffix::None => Output::ComTriOut(term),
            _ => panic!("Nope!"),
        };

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
            Output::Undriven => return Err(ErrorCode::PrematureENABLE),
            Output::RegOut(_) => {
                if gal.chip == Chip::GAL16V8 || gal.chip == Chip::GAL20V8 {
                    return Err(ErrorCode::TristateReg);
                }
            }
            Output::ComTriOut(_) => return Err(ErrorCode::UnmatchedTristate),
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
            Output::Undriven => return Err(ErrorCode::PrematureCLK),
            Output::RegOut(_) => {}
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
            Output::Undriven => return Err(ErrorCode::PrematureARST),
            Output::RegOut(_) => {}
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
            Output::Undriven => return Err(ErrorCode::PrematureAPRST),
            Output::RegOut(_) => {}
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
    let mode = get_mode_v8(gal, olmcs);
    gal.set_mode(mode);
    return mode;
}

pub fn get_mode_v8(gal: &mut gal::GAL, olmcs: &[OLMC]) -> Mode {
    // If there's a registered pin, it's mode 3.
    for n in 0..8 {
        if let Output::RegOut(_) = olmcs[n].output  {
            return Mode::Mode3;
        }
    }
    // If there's a tristate, it's mode 2.
    for n in 0..8 {
        if let Output::TriOut(_) = olmcs[n].output {
            return Mode::Mode2;
        }
    }
    // If we can't use mode 1, use mode 2.
    let chip = gal.chip;
    for n in 0..8 {
        // Some pins cannot be used as input or feedback.
        if olmcs[n].feedback && olmcs[n].output == Output::Undriven {
            if chip == Chip::GAL16V8 {
                let pin_num = n + 12;
                if pin_num == 15 || pin_num == 16 {
                    return Mode::Mode2;
                }
            }
            if chip == Chip::GAL20V8 {
                let pin_num = n + 15;
                if pin_num == 18 || pin_num == 19 {
                    return Mode::Mode2;
                }
            }
        }
        // Other pins cannot be used as feedback.
        if olmcs[n].feedback {
            if let Output::ComTriOut(_) = olmcs[n].output {
                return Mode::Mode2;
            }
        }
    }
    // If there is still no mode defined, use mode 1.
    return Mode::Mode1;
}

pub fn analyse_mode(gal: &mut gal::GAL, olmcs: &mut [OLMC]) -> Option<gal::Mode> {
    match gal.chip {
        Chip::GAL16V8 | Chip::GAL20V8 => {
            let mode = analyse_mode_v8(gal, olmcs);

            for n in 0..8 {
                // Copy the term out, if it's there.
                let term = if let Output::ComTriOut(ref term) = olmcs[n].output {
                    Some(term.clone())
                } else {
                    None
                };

                // And update based on the copied term.
                if let Some(term) = term {
                    if mode == Mode::Mode1 {
                        olmcs[n].output = Output::ComOut(term.clone());
                    } else {
                        olmcs[n].output = Output::TriOut(term.clone());
                        // Set to VCC.
                        olmcs[n].tri_con = Some(gal::true_term(term.line_num));
                    }
                }
            }

            // SYN and AC0 already defined.

            for n in 0..64 {
                gal.pt[n] = true;
            }

            for n in 0..8 {
                if match olmcs[n].output {
                    Output::Undriven => olmcs[n].feedback,
                    Output::TriOut(_) => true,
                    _ => false,
                } {
                    gal.ac1[7 - n] = true;
                }
            }

            for n in 0..8 {
                if olmcs[n].output != Output::Undriven && olmcs[n].active == Active::HIGH {
                    gal.xor[7 - n] = true;
                }
            }

            return Some(mode);
        }

        Chip::GAL22V10 => {
            for n in 0..10 {
                let term = if let Output::ComTriOut(ref term) = olmcs[n].output {
                    Some(term.clone())
                } else {
                    None
                };

                if let Some(term) = term {
                    olmcs[n].output = Output::TriOut(term.clone());
                }

                if olmcs[n].output != Output::Undriven && olmcs[n].active == Active::HIGH {
                    gal.xor[9 - n] = true;
                }

                if match olmcs[n].output {
                    Output::Undriven => olmcs[n].feedback,
                    Output::TriOut(_) => true,
                    _ => false,
                } {
                    gal.s1[9 - n] = true;
                }
            }
        }

        Chip::GAL20RA10 => {
            for n in 0..10 {
                let term = if let Output::ComTriOut(ref term) = olmcs[n].output {
                    Some(term.clone())
                } else {
                    None
                };

                if let Some(term) = term {
                    olmcs[n].output = Output::TriOut(term.clone());
                }

                if olmcs[n].output != Output::Undriven && olmcs[n].active == Active::HIGH {
                    gal.xor[9 - n] = true;
                }
            }
        }
    }

    None
}
