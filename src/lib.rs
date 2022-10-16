//
// lib.rs: The Galette GAL assembly library.
//
// In short, Galette takes a set of equations representing the logic
// you want a GAL to implement, and generates a JEDEC file that can be
// programmed into the GAL in order to make it implement those
// equations.
//
// The galette binary is a thin wrapper around "assemble", but if you
// want to programmatically generate GAL assembly files, you should be
// able to use the publicly exposed members of the library, starting
// from a parser::Content or a blueprint::Blueprint, depending on what
// you want to start with.
//

pub mod blueprint;
pub mod chips;
pub mod errors;
pub mod gal;
pub mod gal_builder;
pub mod parser;
pub mod writer;

pub fn assemble(file_name: &str, config: &writer::Config) -> Result<(), errors::FileError> {
    (|| {
        let content = parser::parse(file_name)?;
        let blueprint = blueprint::Blueprint::from(&content)?;
        let gal = gal_builder::build(&blueprint)?;
        writer::write_files(file_name, config, &blueprint.pins, &blueprint.olmcs, &gal).unwrap();

        Ok(())
    })()
    .map_err(|err| errors::FileError {
        file: file_name.into(),
        err,
    })
}
