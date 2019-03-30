use chips::Chip;
use jedec;
use jedec::Mode;

// Config use on the C side.
#[repr(C)]
#[derive(Debug)]
pub struct OLMC {
    pub active: u8,
    pub pin_type: u8,
    pub tri_con: u8,
    pub clock: u8,
    pub arst: u8,
    pub aprst: u8,
    pub feedback: u8,
}

const NOTUSED    : u8 =     0;             /* pin not used up to now */
const NOTCON     : u8 =     0;             /* pin not used           */
const INPUT      : u8 =     2;             /* input                  */
const COMOUT     : u8 =     3;             /* combinational output   */
const TRIOUT     : u8 =     4;             /* tristate output        */
const REGOUT     : u8 =     5;             /* register output        */
const COM_TRI_OUT: u8 =     6;             /* either tristate or     */


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
