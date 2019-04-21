# Galette: A GAL assembler for the 21st Century

Galette is a GALasm-compatible GAL assembler that takes a set of
equations, and generates a JEDEC file suitable for feeding to a GAL
programmer.

## Usage

The input file format is a slightly relaxed modified of the GALasm
format. Differences include:

 * The "DESCRIPTION" section at the end of the .pld file is now optional.
 * You don't actually need to include any equations at all! All outputs
   are undriven.
 * You can use long pins names, and the only downside is it makes the
   output files lose alignment.
 * Tristate enables etc. no longer need to be defined after the main
   output definition (as long as that definition is present somewhere).
 * Newlines are important.
   * We assume one line per equation.
   * The list of pins must be split equally across two lines.
   * This is much less free-form than galasm, but hopefully makes
     understanding syntax errors a lot easier.
   * Maybe I'll change my mind!

`galette --help` gives you a summary of the (GALasm-compatible)
command-line options.

## More docs

For more docs, refer to [galasm](https://github.com/daveho/GALasm)'s
documentation. I've decided not to include them in this repo in order
to simplify the licensing situation.

## Background

When I say "A GAL assembler for the 21st Century", my tongue's pretty
firmly in my cheek. No-one should really want a GAL assembler
nowadays. This is dead tech. :)

[GALasm](https://github.com/dwery/galasm) was a turn-of-the-century
update of GALer, an early '90s open source GAL assembler for the
Amiga. It's written in C and the style is from another era.

I was trying to program GALs, and having some problems. In the end, it
turned out to be my power supply, but along the way I discovered a
couple of bugs in GALasm and generally felt that it could do with an
overhaul. I've been trying to learn Rust, and so incrementally porting
it to a memory-safe language while refactoring the structure along the
way seemed a fun project.

This is the result. It's a prime example of yak-shaving.

## Source organisation

Running from the lowest layer of dependency to the highest, we have:

 * **errors.rs** Error codes used by everything else.
 * **chips.rs** An abstraction layer over the different GAL types.
 * **gal.rs** Contains the GAL structure with is programmed with fuse data.
 * **parser.rs** Parse the input file format.
 * **blueprint.rs** Convert input to a normalised form to build fuses from.
 * **gal_builder.rs** Fills in a GAL structure based on a blueprint.
 * **writer.rs** Writes out the generated GAL structure.
 * **lib.rs** Top-level glue.
 * **main.rs** Thin command-line wrapper.

## Tests

As I've been trying to maintain the behaviour of galasm, I've
concentrated on end-to-end tests (feed in a file, see what comes out)
rather than unit tests, which has allowed me to refactor the program
without needing to keep any specific internal structure, as long as
the output's the same.

So, if you're wondering why there's no unit tests, that's why.

To run the tests, `./run_tests.sh`.

## Licensing

I'm releasing my code under the MIT license. While I based galette on
galasm, this is a reimplementation of the functionality in a different
language, structured very differently.

I think it's useful to have a clean break here, as the licensing
situation of galasm is not particularly clear, and it's helpful to
have a GAL assembler under an explicit open license.

I am, of course, still extremely indebted to Alessandro Zummo and
Christian Habermann for the starting point of galasm.

## TODOs

Some nice-to-haves I probably won't get around to:

 * Actually check if the checksum logic is broken on overflow (see
   source TODO).
 * Add tests for the cases where we're deliberately different from
   galasm.
  * Specifically, long pin names, no equations, no DESCRIPTION,
    auxiliary equations before main equations.
 * Add coverage testing.
