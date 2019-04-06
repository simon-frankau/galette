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

#[derive(Clone, Debug)]
pub struct OLMC {
    pub active: u8,
    pub pin_type: u8,
    pub output: Option<gal_builder::Equation>,
    pub tri_con: Tri,
    pub clock: Option<gal_builder::Equation>,
    pub arst: Option<gal_builder::Equation>,
    pub aprst: Option<gal_builder::Equation>,
    pub feedback: u8,
}

pub const NOTUSED    : u8 =     0;             /* pin not used up to now */
pub const NOTCON     : u8 =     0;             /* pin not used           */
pub const INPUT      : u8 =     2;             /* input                  */
pub const COMOUT     : u8 =     3;             /* combinational output   */
pub const TRIOUT     : u8 =     4;             /* tristate output        */
pub const REGOUT     : u8 =     5;             /* register output        */
pub const COM_TRI_OUT: u8 =     6;             /* either tristate or     */

pub const ACTIVE_LOW: u8 =      0;             /* pin is high-active */
pub const ACTIVE_HIGH: u8 =     1;             /* pin is low-active  */

// Get the mode for GAL16V8 and GAL20V8, set the flags appropriately
pub fn analyse_mode_v8(jedec: &mut jedec::Jedec, olmcs: &[OLMC]) -> Mode {
    let mode = get_mode_v8(jedec, olmcs);
    jedec.set_mode(mode);
    return mode;
}

pub fn get_mode_v8(jedec: &mut jedec::Jedec, olmcs: &[OLMC]) -> Mode {
    // If there's a registered pin, it's mode 3.
    for n in 0..8 {
        if olmcs[n].pin_type == REGOUT {
            return Mode::Mode3;
        }
    }
    // If there's a tristate, it's mode 2.
    for n in 0..8 {
        if olmcs[n].pin_type == TRIOUT {
            return Mode::Mode2;
        }
    }
    // If we can't use mode 1, use mode 2.
    let chip = jedec.chip;
    for n in 0..8 {
        if olmcs[n].pin_type == INPUT {
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
        if olmcs[n].pin_type == COM_TRI_OUT && olmcs[n].feedback != 0 {
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
                if olmcs[n].pin_type == COM_TRI_OUT {
                    if mode == Mode::Mode1 {
                        olmcs[n].pin_type = COMOUT;
                    } else {
                        olmcs[n].pin_type = TRIOUT;
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
                if olmcs[n].pin_type == INPUT || olmcs[n].pin_type == TRIOUT {
                    jedec.ac1[7 - n] = true;
                }
            }

            for n in 0..8 {
                if ((olmcs[n].pin_type == COMOUT) || (olmcs[n].pin_type == TRIOUT) || (olmcs[n].pin_type == REGOUT)) && (olmcs[n].active == ACTIVE_HIGH) {
                    jedec.xor[7 - n] = true;
                }
            }

            return Some(mode);
        }

        Chip::GAL22V10 => {
            for n in 0..10 {
                if olmcs[n].pin_type == COM_TRI_OUT {
                    olmcs[n].pin_type = TRIOUT;
                }

                if ((olmcs[n].pin_type == COMOUT) || (olmcs[n].pin_type == TRIOUT) || (olmcs[n].pin_type == REGOUT)) && (olmcs[n].active == ACTIVE_HIGH) {
                    jedec.xor[9 - n] = true;
                }

                if olmcs[n].pin_type == INPUT || olmcs[n].pin_type == TRIOUT {
                    jedec.s1[9 - n] = true;
                }
            }
        }

        Chip::GAL20RA10 => {
            for n in 0..10 {
                if olmcs[n].pin_type == COM_TRI_OUT {
                    olmcs[n].pin_type = TRIOUT;
                }

                if ((olmcs[n].pin_type == COMOUT) || (olmcs[n].pin_type == TRIOUT) || (olmcs[n].pin_type == REGOUT)) && (olmcs[n].active == ACTIVE_HIGH) {
                    jedec.xor[9 - n] = true;
                }
            }
        }
    }

    None
}
