// TODO: Stub main entry point, to replace galasm.cpp.

extern crate galette;

use std::env;
use std::process;

use galette::blueprint;
use galette::errors;
use galette::jedec_writer;
use galette::gal_builder;
use galette::parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_name = &args[1];

    let config = jedec_writer::Config {
        gen_fuse: 1,
        gen_chip: 1,
        gen_pin: 1,
        jedec_sec_bit: 0,
        jedec_fuse_chk: 1,
    };

    let c = match parser::parse(file_name) {
        Ok(c) => c,
        Err(e) => { errors::print_error(e); process::exit(1);; }
    };

    let mut blueprint = match blueprint::Blueprint::from(&c) {
        Ok(b) => b,
        Err(e) => { errors::print_error(e); process::exit(1); }
    };

    match gal_builder::do_stuff(&mut blueprint, file_name, &config) {
        Ok(()) => (),
        Err(e) => { errors::print_error(e); process::exit(1); }
    };
}