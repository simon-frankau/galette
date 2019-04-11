use chips::Chip;
use errors::ErrorCode;
use gal_builder;
use gal;
use gal::GAL;
use gal::Term;
use olmc;
use olmc::OLMC;
use olmc::PinType;
use parser::Equation;

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
            pin_type: PinType::UNDRIVEN,
            output: None,
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
            if let Some(i) = gal.chip.pin_to_olmc(input.pin as usize) {
                olmcs[i].feedback = true;
            }
        }

        let term = eqn_to_term(gal.chip, &eqn)?;

        // AR/SP special cases:
        match act_pin.pin {
            24 => {
                if self.ar.is_some() {
                    return Err(ErrorCode::REPEATED_AR_SP);
                }
                self.ar = Some(term); return Ok(());
            }
            25 => {
                if self.sp.is_some() {
                    return Err(ErrorCode::REPEATED_AR_SP);
                }
                self.sp = Some(term); return Ok(());
            }
            _ => {}
        }

        // Only pins with OLMCs may be outputs.
        let olmc_num = match gal.chip.pin_to_olmc(act_pin.pin as usize) {
            None => return Err(ErrorCode::NOT_AN_OUTPUT),
            Some(i) => i,
        };
        let olmc = &mut olmcs[olmc_num];

        match eqn.suffix {
            gal_builder::SUFFIX_R | gal_builder::SUFFIX_T | gal_builder::SUFFIX_NON =>
                olmc.set_base(act_pin, term, eqn.suffix),
            gal_builder::SUFFIX_E =>
                olmc.set_enable(gal, act_pin, term),
            gal_builder::SUFFIX_CLK =>
                olmc.set_clock(act_pin, term),
            gal_builder::SUFFIX_ARST =>
                olmc.set_arst(act_pin, term),
            gal_builder::SUFFIX_APRST =>
                olmc.set_aprst(act_pin, term),
            _ =>
                panic!("Nope"),
        }
    }
}

fn eqn_to_term(chip: Chip, eqn: &Equation) -> Result<Term, ErrorCode> {
    if eqn.rhs.len() == 1 {
        let pin = &eqn.rhs[0];
        if pin.pin as usize == chip.num_pins() {
            // VCC
            if pin.neg != 0 {
                return Err(ErrorCode::INVERTED_POWER);
            }
            return Ok(gal::true_term(eqn.line_num));
        } else if pin.pin as usize == chip.num_pins() / 2 {
            // GND
            if pin.neg != 0 {
                return Err(ErrorCode::INVERTED_POWER);
            }
            return Ok(gal::false_term(eqn.line_num));
        }
    }

    let mut ors = Vec::new();
    let mut ands = Vec::new();

    for (pin, op) in eqn.rhs.iter().zip(eqn.ops.iter()) {
        if *op == 43 || *op == 35 {
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
