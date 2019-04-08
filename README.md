# Galette: A GAL assembler for the 21st Century

Galette is a GALasm-compatible GAL assembler that takes a set of
equations, and generates a JEDEC file suitable for feeding to a GAL
programmer.

## Usage

TODO!

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

This is the result.

## Source organisation

Running from the lowest layer of dependency to the highest, we have:

 * **chips.rs** An abstraction layer over the different GAL types.
 * TODO

## TODOs

 * Better documentation. :)
 * Better error-handling
 * Built in parser
 * Set of test cases
