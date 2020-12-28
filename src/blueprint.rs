use errors::OutputSuffix;

//
// blueprint.rs: Assembly-ready representation
//
// Blueprint is an intermediate form where the initial equations are
// converted into a form that are ready to be made into fuse maps.
// Each output pin is configured via an "OLMC" data structure.
//
use crate::{
    chips::Chip,
    errors::{self, Error, ErrorCode},
    gal::{self, Pin, Term},
    parser::{Content, Equation, Suffix, LHS},
};

// Blueprint stores everything we need to construct the GAL.
pub struct Blueprint {
    // Data copied straight over from parser::Content.
    pub chip: Chip,
    pub sig: Vec<u8>,
    pub pins: Vec<String>,
    // The Equations, transformed.
    pub olmcs: Vec<OLMC>,
    // GAL22V10 only:
    pub ar: Option<Term>,
    pub sp: Option<Term>,
}

impl Blueprint {
    pub fn new(chip: Chip) -> Self {
        // Set up OLMCs.
        let olmcs = vec![
            OLMC {
                active: Active::Low,
                output: None,
                tri_con: None,
                clock: None,
                arst: None,
                aprst: None,
                feedback: false,
            };
            chip.num_olmcs()
        ];

        Blueprint {
            chip,
            sig: Vec::new(),
            pins: Vec::new(),
            olmcs,
            ar: None,
            sp: None,
        }
    }

    pub fn from(content: &Content) -> Result<Self, Error> {
        let mut blueprint = Blueprint::new(content.chip);

        // Convert equations into data on the OLMCs.
        for eqn in content.eqns.iter() {
            errors::at_line(eqn.line_num, blueprint.add_equation(eqn))?;
        }

        blueprint.sig = content.sig.clone();
        blueprint.pins = content.pins.clone();

        Ok(blueprint)
    }

    // Add an equation to the blueprint, steering it to the appropriate OLMC.
    pub fn add_equation(&mut self, eqn: &Equation) -> Result<(), ErrorCode> {
        let olmcs = &mut self.olmcs;

        // Mark all OLMCs that are inputs to other equations as providing feedback.
        // (Note they may actually be used as undriven inputs.)
        for input in eqn.rhs.iter() {
            if let Some(i) = self.chip.pin_to_olmc(input.pin) {
                olmcs[i].feedback = true;
            }
        }

        let term = eqn_to_term(self.chip, &eqn)?;

        // AR/SP special cases:
        match eqn.lhs {
            LHS::Ar => {
                if self.ar.is_some() {
                    return Err(ErrorCode::RepeatedARSP);
                }
                self.ar = Some(term);
            }
            LHS::Sp => {
                if self.sp.is_some() {
                    return Err(ErrorCode::RepeatedARSP);
                }
                self.sp = Some(term);
            }
            LHS::Pin((pin, suffix)) => {
                // Only pins with OLMCs may be outputs.
                let olmc_num = self
                    .chip
                    .pin_to_olmc(pin.pin)
                    .ok_or(ErrorCode::NotAnOutput)?;
                let olmc = &mut olmcs[olmc_num];

                match suffix {
                    Suffix::R => olmc.set_base(&pin, term, PinMode::Registered),
                    Suffix::None => olmc.set_base(&pin, term, PinMode::Combinatorial),
                    Suffix::T => olmc.set_base(&pin, term, PinMode::Tristate),
                    Suffix::E => olmc.set_enable(&pin, term),
                    Suffix::CLK => olmc.set_clock(&pin, term),
                    Suffix::ARST => olmc.set_arst(&pin, term),
                    Suffix::APRST => olmc.set_aprst(&pin, term),
                }?;
            }
        }

        Ok(())
    }
}

// Convert an Equation, which is close to the input syntax, into a
// Term, which is close to the fuse map representation.
fn eqn_to_term(chip: Chip, eqn: &Equation) -> Result<Term, ErrorCode> {
    // Special case for constant true or false.
    if eqn.rhs.len() == 1 {
        let pin = &eqn.rhs[0];
        if pin.pin == chip.num_pins() {
            // VCC
            if pin.neg {
                return Err(ErrorCode::InvertedPower);
            }
            return Ok(gal::true_term(eqn.line_num));
        } else if pin.pin == chip.num_pins() / 2 {
            // GND
            if pin.neg {
                return Err(ErrorCode::InvertedPower);
            }
            return Ok(gal::false_term(eqn.line_num));
        }
    }

    // Create a list of OR'd terms, each team being a group of AND'd terms.
    let mut ors = Vec::new();
    let mut ands = Vec::new();

    for (pin, is_or) in eqn.rhs.iter().zip(eqn.is_or.iter()) {
        if *is_or {
            ors.push(ands);
            ands = Vec::new();
        }
        ands.push(*pin);
    }
    ors.push(ands);

    Ok(Term {
        line_num: eqn.line_num,
        pins: ors,
    })
}

////////////////////////////////////////////////////////////////////////
// The OLMC structure, representing the logic for an output pin.
//

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

#[derive(Clone, Debug, PartialEq)]
pub enum Active {
    Low,
    High,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PinMode {
    Combinatorial,
    Tristate,
    Registered,
}

impl OLMC {
    pub fn set_base(&mut self, pin: &Pin, term: Term, pin_mode: PinMode) -> Result<(), ErrorCode> {
        if self.output.is_some() {
            // Previously defined, so error out.
            return Err(ErrorCode::RepeatedOutput);
        }
        self.output = Some((pin_mode, term));

        self.active = if pin.neg { Active::Low } else { Active::High };

        Ok(())
    }

    pub fn set_enable(&mut self, pin: &Pin, term: Term) -> Result<(), ErrorCode> {
        if pin.neg {
            return Err(ErrorCode::InvertedControl {
                suffix: OutputSuffix::E,
            });
        }

        if self.tri_con != None {
            return Err(ErrorCode::RepeatedControl {
                suffix: OutputSuffix::E,
            });
        }
        self.tri_con = Some(term);

        Ok(())
    }

    pub fn set_clock(&mut self, pin: &Pin, term: Term) -> Result<(), ErrorCode> {
        if pin.neg {
            return Err(ErrorCode::InvertedControl {
                suffix: OutputSuffix::CLK,
            });
        }

        if self.clock.is_some() {
            return Err(ErrorCode::RepeatedControl {
                suffix: OutputSuffix::CLK,
            });
        }
        self.clock = Some(term);

        Ok(())
    }

    pub fn set_arst(&mut self, pin: &Pin, term: Term) -> Result<(), ErrorCode> {
        if pin.neg {
            return Err(ErrorCode::InvertedControl {
                suffix: OutputSuffix::ARST,
            });
        }

        if self.arst.is_some() {
            return Err(ErrorCode::RepeatedControl {
                suffix: OutputSuffix::ARST,
            });
        }
        self.arst = Some(term);

        Ok(())
    }

    pub fn set_aprst(&mut self, pin: &Pin, term: Term) -> Result<(), ErrorCode> {
        if pin.neg {
            return Err(ErrorCode::InvertedControl {
                suffix: OutputSuffix::APRST,
            });
        }

        if self.aprst.is_some() {
            return Err(ErrorCode::RepeatedControl {
                suffix: OutputSuffix::APRST,
            });
        }
        self.aprst = Some(term);

        Ok(())
    }
}
