//
// main.rs: Entry point for the galette binary.
//
// While galette is written to be usable as a library, it also
// provides a command-line interface that is intended to be largely
// compatible with galette's.
//

extern crate clap;
extern crate galette;

use clap::{App, Arg};

use std::process;

use galette::writer;

fn main() {
    let matches = App::new("Galette")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Simon Frankau <sgf@arbitrary.name>")
        .about("GALasm-compatible GAL assembler")
        .arg(
            Arg::with_name("INPUT.pld")
                .help("Input file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("secure")
                .short("s")
                .long("secure")
                .takes_value(false)
                .help("Enable security fuse"),
        )
        .arg(
            Arg::with_name("nochip")
                .short("c")
                .long("nochip")
                .takes_value(false)
                .help("Disable .chp file output"),
        )
        .arg(
            Arg::with_name("nofuse")
                .short("f")
                .long("nofuse")
                .takes_value(false)
                .help("Disable .fus file output"),
        )
        .arg(
            Arg::with_name("nopin")
                .short("p")
                .long("nopin")
                .takes_value(false)
                .help("Disable .pin file output"),
        )
        .get_matches();

    let file_name = matches.value_of("INPUT.pld").unwrap();

    let config = writer::Config {
        gen_fuse: !matches.is_present("nofuse"),
        gen_chip: !matches.is_present("nochip"),
        gen_pin: !matches.is_present("nopin"),
        jedec_sec_bit: matches.is_present("secure"),
    };

    if let Err(e) = galette::assemble(&file_name, &config) {
        eprintln!("{}", e);
        process::exit(1);
    }
}
