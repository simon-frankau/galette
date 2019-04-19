use chips::Chip;
use errors::ErrorCode;
use parser::Suffix;
use gal;
use gal::GAL;
use gal::Term;
use olmc;
use olmc::OLMC;
use olmc::Output;
use parser::Equation;
use parser::LHS;

// Blueprint stores everything we need to construct the GAL.
pub struct Blueprint {
    pub olmcs: Vec<OLMC>,
    // GAL22V10 only:
    pub ar: Option<Term>,
    pub sp: Option<Term>,
}

impl Blueprint {
    pub fn new(chip: Chip) -> Self {
        // Set up OLMCs.
        let olmcs = vec!(OLMC {
            active: olmc::Active::LOW,
            output: Output::Undriven,
            tri_con: None,
            clock: None,
            arst: None,
            aprst: None,
            feedback: false,
         }; chip.num_olmcs());

         Blueprint {
             olmcs: olmcs,
             ar: None,
             sp: None,
         }
    }

    // Add an equation to the blueprint, steering it to the appropriate OLMC.
    pub fn add_equation(
        &mut self,
        eqn: &Equation,
        gal: &GAL,
    ) -> Result<(), ErrorCode> {
        let olmcs = &mut self.olmcs;
        let act_pin = &eqn.lhs;

        // Mark all OLMCs that are inputs to other equations as providing feedback.
        // (Note they may actually be used as undriven inputs.)
        for input in eqn.rhs.iter() {
            if let Some(i) = gal.chip.pin_to_olmc(input.pin) {
                olmcs[i].feedback = true;
            }
        }

        let term = eqn_to_term(gal.chip, &eqn)?;

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
                let olmc_num = match gal.chip.pin_to_olmc(act_pin.pin) {
                    None => return Err(ErrorCode::NotAnOutput),
                    Some(i) => i,
                };
                let olmc = &mut olmcs[olmc_num];

                match *suffix {
                    Suffix::R | Suffix::T | Suffix::None =>
                        olmc.set_base(act_pin, term, *suffix),
                    Suffix::E =>
                        olmc.set_enable(gal, act_pin, term),
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
