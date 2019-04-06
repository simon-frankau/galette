use chips::Chip;
use gal_builder;
use jedec;
use jedec::Mode;

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
    pub output: Option<gal_builder::Equation>,
    pub tri_con: Tri,
    pub clock: Option<gal_builder::Equation>,
    pub arst: Option<gal_builder::Equation>,
    pub aprst: Option<gal_builder::Equation>,
    pub feedback: bool,
}

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
                if ((olmcs[n].pin_type == PinType::COMOUT) || (olmcs[n].pin_type == PinType::TRIOUT) || (olmcs[n].pin_type == PinType::REGOUT)) && (olmcs[n].active == Active::HIGH) {
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

                if ((olmcs[n].pin_type == PinType::COMOUT) || (olmcs[n].pin_type == PinType::TRIOUT) || (olmcs[n].pin_type == PinType::REGOUT)) && (olmcs[n].active == Active::HIGH) {
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

                if ((olmcs[n].pin_type == PinType::COMOUT) || (olmcs[n].pin_type == PinType::TRIOUT) || (olmcs[n].pin_type == PinType::REGOUT)) && (olmcs[n].active == Active::HIGH) {
                    jedec.xor[9 - n] = true;
                }
            }
        }
    }

    None
}
