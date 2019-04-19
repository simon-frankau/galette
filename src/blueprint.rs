use chips::Chip;
use errors;
use errors::Error;
use errors::ErrorCode;
use gal;
use gal::Term;
use olmc;
use olmc::OLMC;
use parser::Content;
use parser::Equation;
use parser::LHS;
use parser::Suffix;

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
        let olmcs = vec!(OLMC {
            active: olmc::Active::Low,
            output: None,
            tri_con: None,
            clock: None,
            arst: None,
            aprst: None,
            feedback: false,
         }; chip.num_olmcs());

         Blueprint {
             chip: chip,
             sig: Vec::new(),
             pins: Vec::new(),
             olmcs: olmcs,
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
    pub fn add_equation(
        &mut self,
        eqn: &Equation,
    ) -> Result<(), ErrorCode> {
        let olmcs = &mut self.olmcs;
        let act_pin = &eqn.lhs;

        // Mark all OLMCs that are inputs to other equations as providing feedback.
        // (Note they may actually be used as undriven inputs.)
        for input in eqn.rhs.iter() {
            if let Some(i) = self.chip.pin_to_olmc(input.pin) {
                olmcs[i].feedback = true;
            }
        }

        let term = eqn_to_term(self.chip, &eqn)?;

        // AR/SP special cases:
        match act_pin {
            LHS::Ar => {
                if self.ar.is_some() {
                    return Err(ErrorCode::RepeatedARSP);
                }
                self.ar = Some(term);
                 Ok(())
            }
            LHS::Sp => {
                if self.sp.is_some() {
                    return Err(ErrorCode::RepeatedARSP);
                }
                self.sp = Some(term);
                Ok(())
            }
            LHS::Pin((act_pin, suffix)) => {
                // Only pins with OLMCs may be outputs.
                let olmc_num = match self.chip.pin_to_olmc(act_pin.pin) {
                    None => return Err(ErrorCode::NotAnOutput),
                    Some(i) => i,
                };
                let olmc = &mut olmcs[olmc_num];

                match *suffix {
                    Suffix::R | Suffix::T | Suffix::None =>
                        olmc.set_base(act_pin, term, *suffix),
                    Suffix::E =>
                        olmc.set_enable(self.chip, act_pin, term),
                    Suffix::CLK =>
                        olmc.set_clock(act_pin, term),
                    Suffix::ARST =>
                        olmc.set_arst(act_pin, term),
                    Suffix::APRST =>
                        olmc.set_aprst(act_pin, term),
                }
            }
        }
    }
}

// Convert an Equation, which is close to the input syntax, into a
// Term, which is close to the fuse map representation.
fn eqn_to_term(chip: Chip, eqn: &Equation) -> Result<Term, ErrorCode> {
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
