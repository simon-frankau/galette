use chips::Chip;
use errors::ErrorCode;
use gal_builder;
use gal_builder::Pin;
use gal;
use gal::GAL;
use gal::Mode;
use gal::Term;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PinType {
    UNDRIVEN,
    COMOUT,
    TRIOUT,
    REGOUT,
    COMTRIOUT,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Active {
    LOW,
    HIGH
}

#[derive(Clone, Debug)]
pub struct OLMC {
    pub active: Active,
    pub pin_type: PinType,
    pub output: Option<gal::Term>,
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
//  -> TRIOUT - tristate
//  -> REGOUT - registered
//  -> COMTRIOUT - combinatorial, might be tristated.
//     analysed to:
//     -> COM_OUT
//     -> TRI_OUT

impl OLMC {
    pub fn set_base(
        &mut self,
        act_pin: &Pin,
        term: Term,
        suffix: i32,
    ) -> Result<(), ErrorCode> {
        if self.output.is_some() {
            // Previously defined, so error out.
            return Err(ErrorCode::Code(16));
        }

        self.output = Some(term);

        self.active = if act_pin.neg != 0 {
            Active::LOW
        } else {
            Active::HIGH
        };

        self.pin_type = match suffix {
            gal_builder::SUFFIX_T => PinType::TRIOUT,
            gal_builder::SUFFIX_R => PinType::REGOUT,
            gal_builder::SUFFIX_NON => PinType::COMTRIOUT,
            _ => panic!("Nope!"),
        };

        Ok(())
    }

    pub fn set_enable(
        &mut self,
        gal: &GAL,
        act_pin: &Pin,
        term: Term,
    ) -> Result<(), ErrorCode> {
        if act_pin.neg != 0 {
            return Err(ErrorCode::Code(19));
        }

        if self.tri_con != None {
            return Err(ErrorCode::Code(22));
        }

        self.tri_con = Some(term);

        if self.pin_type == PinType::UNDRIVEN {
            return Err(ErrorCode::Code(17));
        }

        if self.pin_type == PinType::REGOUT && (gal.chip == Chip::GAL16V8 || gal.chip == Chip::GAL20V8) {
            return Err(ErrorCode::Code(23));
        }

        if self.pin_type == PinType::COMTRIOUT {
            return Err(ErrorCode::Code(24));
        }

        Ok(())
    }

    pub fn set_clock(
        &mut self,
        act_pin: &Pin,
        term: Term,
    ) -> Result<(), ErrorCode> {
        if act_pin.neg != 0 {
            return Err(ErrorCode::Code(19));
        }

        if self.pin_type == PinType::UNDRIVEN {
            return Err(ErrorCode::Code(42));
        }

        if self.clock.is_some() {
            return Err(ErrorCode::Code(45));
        }

        self.clock = Some(term);
        if self.pin_type != PinType::REGOUT {
            return Err(ErrorCode::Code(48));
        }

        Ok(())
    }

    pub fn set_arst(
        &mut self,
        act_pin: &Pin,
        term: Term
    ) -> Result<(), ErrorCode> {
        if act_pin.neg != 0 {
            return Err(ErrorCode::Code(19));
        }

        if self.pin_type == PinType::UNDRIVEN {
            return Err(ErrorCode::Code(43));
        }

        if self.arst.is_some() {
            return Err(ErrorCode::Code(46));
        }

        self.arst = Some(term);
        if self.pin_type != PinType::REGOUT {
            return Err(ErrorCode::Code(48));
        }

        Ok(())
    }

    pub fn set_aprst(
        &mut self,
        act_pin: &Pin,
        term: Term,
    ) -> Result<(), ErrorCode> {
        if act_pin.neg != 0 {
            return Err(ErrorCode::Code(19));
        }

        if self.pin_type == PinType::UNDRIVEN {
            return Err(ErrorCode::Code(44));
        }

        if self.aprst.is_some() {
            return Err(ErrorCode::Code(47));
        }

        self.aprst = Some(term);
        if self.pin_type != PinType::REGOUT {
            return Err(ErrorCode::Code(48));
        }

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
        if olmcs[n].pin_type == PinType::REGOUT {
            return Mode::Mode3;
        }
    }
    // If there's a tristate, it's mode 2.
    for n in 0..8 {
        if olmcs[n].pin_type == PinType::TRIOUT {
            return Mode::Mode2;
        }
    }
    // If we can't use mode 1, use mode 2.
    let chip = gal.chip;
    for n in 0..8 {
        // Some pins cannot be used as input or feedback.
        if olmcs[n].feedback && olmcs[n].pin_type == PinType::UNDRIVEN {
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
        if olmcs[n].feedback && olmcs[n].pin_type == PinType::COMTRIOUT {
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

            for n in 0..8 {
                if olmcs[n].pin_type == PinType::COMTRIOUT {
                    if mode == Mode::Mode1 {
                        olmcs[n].pin_type = PinType::COMOUT;
                    } else {
                        olmcs[n].pin_type = PinType::TRIOUT;
                        // Set to VCC.
                        olmcs[n].tri_con = Some(gal::true_term(olmcs[n].output.as_ref().unwrap().line_num));
                    }
                }
            }

            // SYN and AC0 already defined.

            for n in 0..64 {
                gal.pt[n] = true;
            }

            for n in 0..8 {
                if (olmcs[n].pin_type == PinType::UNDRIVEN && olmcs[n].feedback) || olmcs[n].pin_type == PinType::TRIOUT {
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
                if olmcs[n].pin_type == PinType::COMTRIOUT {
                    olmcs[n].pin_type = PinType::TRIOUT;
                }

                if olmcs[n].output.is_some() && olmcs[n].active == Active::HIGH {
                    gal.xor[9 - n] = true;
                }

                if (olmcs[n].pin_type == PinType::UNDRIVEN && olmcs[n].feedback) || olmcs[n].pin_type == PinType::TRIOUT {
                    gal.s1[9 - n] = true;
                }
            }
        }

        Chip::GAL20RA10 => {
            for n in 0..10 {
                if olmcs[n].pin_type == PinType::COMTRIOUT {
                    olmcs[n].pin_type = PinType::TRIOUT;
                }

                if olmcs[n].output.is_some() && olmcs[n].active == Active::HIGH {
                    gal.xor[9 - n] = true;
                }
            }
        }
    }

    None
}
