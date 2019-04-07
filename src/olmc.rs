use chips::Chip;
use gal_builder;
use gal_builder::Equation;
use gal_builder::Pin;
use jedec;
use jedec::Jedec;
use jedec::Mode;
use jedec::Term;

#[derive(Clone, Debug, PartialEq)]
pub enum Tri {
    None,
    Some(gal_builder::Equation),
    VCC
}

#[derive(Clone, Debug, PartialEq)]
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
    pub output: Option<jedec::Term>,
    pub tri_con: Tri,
    pub clock: Option<jedec::Term>,
    pub arst: Option<jedec::Term>,
    pub aprst: Option<jedec::Term>,
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
        jedec: &Jedec,
        act_pin: &Pin,
        is_arsp: bool, // TODO: Hack for the error message?
        eqn: &Equation,
    ) -> Result<(), i32> {
        if self.output.is_some() {
            // Previously defined, so error out.
            if jedec.chip == Chip::GAL22V10 && is_arsp {
                return Err(40);
            } else {
                return Err(16);
            }
        }

        self.output = Some(eqn_to_term(eqn));

        self.active = if act_pin.neg != 0 {
            Active::LOW
        } else {
            Active::HIGH
        };

        self.pin_type = match eqn.suffix {
            gal_builder::SUFFIX_T => PinType::TRIOUT,
            gal_builder::SUFFIX_R => PinType::REGOUT,
            gal_builder::SUFFIX_NON => PinType::COMTRIOUT,
            _ => panic!("Nope!"),
        };

        Ok(())
    }

    pub fn set_enable(
        &mut self,
        jedec: &Jedec,
        act_pin: &Pin,
        eqn: &Equation,
    ) -> Result<(), i32> {
        if act_pin.neg != 0 {
            return Err(19);
        }

        if self.tri_con != Tri::None {
            return Err(22);
        }

        self.tri_con = Tri::Some(*eqn);

        if self.pin_type == PinType::UNDRIVEN {
            return Err(17);
        }

        if self.pin_type == PinType::REGOUT && (jedec.chip == Chip::GAL16V8 || jedec.chip == Chip::GAL20V8) {
            return Err(23);
        }

        if self.pin_type == PinType::COMTRIOUT {
            return Err(24);
        }

        Ok(())
    }

    pub fn set_clock(
        &mut self,
        act_pin: &Pin,
        eqn: &Equation,
    ) -> Result<(), i32> {
        if act_pin.neg != 0 {
            return Err(19);
        }

        if self.pin_type == PinType::UNDRIVEN {
            return Err(42);
        }

        if self.clock.is_some() {
            return Err(45);
        }

        self.clock = Some(eqn_to_term(eqn));
        if self.pin_type != PinType::REGOUT {
            return Err(48);
        }

        Ok(())
    }

    pub fn set_arst(
        &mut self,
        act_pin: &Pin,
        eqn: &Equation
    ) -> Result<(), i32> {
        if act_pin.neg != 0 {
            return Err(19);
        }

        if self.pin_type == PinType::UNDRIVEN {
            return Err(43);
        }

        if self.arst.is_some() {
            return Err(46);
        }

        self.arst = Some(eqn_to_term(eqn));
        if self.pin_type != PinType::REGOUT {
            return Err(48);
        }

        Ok(())
    }

    pub fn set_aprst(
        &mut self,
        act_pin: &Pin,
        eqn: &Equation,
    ) -> Result<(), i32> {
        if act_pin.neg != 0 {
            return Err(19);
        }

        if self.pin_type == PinType::UNDRIVEN {
            return Err(44);
        }

        if self.aprst.is_some() {
            return Err(47);
        }

        self.aprst = Some(eqn_to_term(eqn));
        if self.pin_type != PinType::REGOUT {
            return Err(48);
        }

        Ok(())
    }
}

fn eqn_to_term(eqn: &Equation) -> Term {
    let rhs = unsafe { std::slice::from_raw_parts(eqn.rhs, eqn.num_rhs as usize) };
    let ops = unsafe { std::slice::from_raw_parts(eqn.ops, eqn.num_rhs as usize) };

    jedec::Term {
        line_num: eqn.line_num,
        rhs: rhs.iter().map(|x| *x).collect(),
        ops: ops.iter().map(|x| *x).collect(),
    }
}

////////////////////////////////////////////////////////////////////////
// Analyse OLMCs

// Get the mode for GAL16V8 and GAL20V8, set the flags appropriately
pub fn analyse_mode_v8(jedec: &mut jedec::Jedec, olmcs: &[OLMC]) -> Mode {
    let mode = get_mode_v8(jedec, olmcs);
    jedec.set_mode(mode);
    return mode;
}

pub fn get_mode_v8(jedec: &mut jedec::Jedec, olmcs: &[OLMC]) -> Mode {
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
    let chip = jedec.chip;
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

pub fn analyse_mode(jedec: &mut jedec::Jedec, olmcs: &mut [OLMC]) -> Option<jedec::Mode> {
    match jedec.chip {
        Chip::GAL16V8 | Chip::GAL20V8 => {
            let mode = analyse_mode_v8(jedec, olmcs);

            for n in 0..8 {
                if olmcs[n].pin_type == PinType::COMTRIOUT {
                    if mode == Mode::Mode1 {
                        olmcs[n].pin_type = PinType::COMOUT;
                    } else {
                        olmcs[n].pin_type = PinType::TRIOUT;
                        // Set to VCC.
                        olmcs[n].tri_con = Tri::VCC;
                    }
                }
            }

            // SYN and AC0 already defined.

            for n in 0..64 {
                jedec.pt[n] = true;
            }

            for n in 0..8 {
                if (olmcs[n].pin_type == PinType::UNDRIVEN && olmcs[n].feedback) || olmcs[n].pin_type == PinType::TRIOUT {
                    jedec.ac1[7 - n] = true;
                }
            }

            for n in 0..8 {
                if olmcs[n].output.is_some() && olmcs[n].active == Active::HIGH {
                    jedec.xor[7 - n] = true;
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
                    jedec.xor[9 - n] = true;
                }

                if (olmcs[n].pin_type == PinType::UNDRIVEN && olmcs[n].feedback) || olmcs[n].pin_type == PinType::TRIOUT {
                    jedec.s1[9 - n] = true;
                }
            }
        }

        Chip::GAL20RA10 => {
            for n in 0..10 {
                if olmcs[n].pin_type == PinType::COMTRIOUT {
                    olmcs[n].pin_type = PinType::TRIOUT;
                }

                if olmcs[n].output.is_some() && olmcs[n].active == Active::HIGH {
                    jedec.xor[9 - n] = true;
                }
            }
        }
    }

    None
}
