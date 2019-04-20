pub mod blueprint;
pub mod chips;
pub mod errors;
pub mod gal;
pub mod gal_builder;
pub mod jedec_writer;
pub mod olmc;
pub mod parser;
pub mod writer;

pub fn assemble(file_name: &str, config: &jedec_writer::Config) -> Result<(), errors::Error> {
    let content = parser::parse(file_name)?;
    let blueprint = blueprint::Blueprint::from(&content)?;
    let gal = gal_builder::build(&blueprint)?;
    writer::write_files(file_name, config, &blueprint.pins, &blueprint.olmcs, &gal).unwrap();

    Ok(())
}