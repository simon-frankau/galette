use gal_builder;
use gal_builder::Equation;
use jedec::Jedec;
use olmc;
use olmc::OLMC;
use olmc::PinType;

// Blueprint stores everything we need to construct the GAL.
pub struct Blueprint {
    pub olmcs: Vec<OLMC>,
}

impl Blueprint {
    pub fn new() -> Self {
        // Set up OLMCs.
        let olmcs = vec!(OLMC {
            active: olmc::Active::LOW,
            pin_type: PinType::UNDRIVEN,
            output: None,
            tri_con: olmc::Tri::None,
            clock: None,
            arst: None,
            aprst: None,
            feedback: false,
         };12);

         Blueprint {
             olmcs: olmcs,
         }
    }

    // Add an equation to the blueprint, steering it to the appropriate OLMC
    pub fn add_equation(
        &mut self,
        eqn: &Equation,
        jedec: &Jedec,
    ) -> Result<(), i32> {
        let olmcs = &mut self.olmcs;
        let act_pin = &eqn.lhs;

        // Only pins with OLMCs may be outputs.
        let olmc = match jedec.chip.pin_to_olmc(act_pin.pin as usize) {
            None => return Err(15),
            Some(olmc) => olmc,
        };

        // Mark all OLMCs that are inputs to other equations as providing feedback.
        // (Note they may actually be used as undriven inputs.)
        let rhs = unsafe { std::slice::from_raw_parts(eqn.rhs, eqn.num_rhs as usize) };
        for input in rhs.iter() {
            if let Some(n) = jedec.chip.pin_to_olmc(input.pin as usize) {
                olmcs[n].feedback = true;
            }
        }

        match eqn.suffix {
            gal_builder::SUFFIX_R | gal_builder::SUFFIX_T | gal_builder::SUFFIX_NON =>
                olmc::register_output_base(jedec, &mut olmcs[olmc], act_pin, olmc >= 10, eqn),
            gal_builder::SUFFIX_E =>
                olmc::register_output_enable(jedec, &mut olmcs[olmc], act_pin, eqn),
            gal_builder::SUFFIX_CLK =>
                olmc::register_output_clock(&mut olmcs[olmc], act_pin, eqn),
            gal_builder::SUFFIX_ARST =>
                olmc::register_output_arst(&mut olmcs[olmc], act_pin, eqn),
            gal_builder::SUFFIX_APRST =>
                olmc::register_output_aprst(&mut olmcs[olmc], act_pin, eqn),
            _ =>
                panic!("Nope"),
        }
    }
}
