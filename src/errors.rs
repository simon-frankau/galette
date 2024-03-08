//
// errors.rs: Error codes
//
// Using error codes allows us to have a nice API, do
// internationalisation, etc. So, we define the error codes here. We
// have the error codes, and an error structure that combines the
// error code with the line number.
//

use std::{fmt, str::FromStr};

use thiserror::Error;

pub type LineNum = usize;

#[derive(Clone, Debug, Error)]
#[error("{}: {}", file, err)]
pub struct FileError {
    pub file: String,
    pub err: Error,
}

#[derive(Clone, Debug, Error)]
#[error("Error in line {}: {}", line, code)]
pub struct Error {
    pub code: ErrorCode,
    pub line: LineNum,
}

#[derive(Clone, Debug, Error)]
pub enum ErrorCode {
    #[error("GAL22V10: {term} is not allowed as pinname")]
    ReservedPinName { term: SpecialProductTerm },
    #[error("no suffix is allowed for {term}")]
    SpecialSuffix { term: SpecialProductTerm },
    #[error("internal error: analyse_mode should never let you use this pin as an input")]
    BadAnalysis,
    #[error("use of {term} is not allowed in equations")]
    BadSpecial { term: SpecialProductTerm },
    #[error("unexpected character in input: '{c}'")]
    BadChar { c: char },
    #[error("expected right-hand side of equation, found end of file")]
    BadEquationEOF,
    #[error("expected pin name, found end of line")]
    BadEOL,
    #[error("unexpected GAL type found: '{gal}'")]
    BadGALType { gal: String },
    #[error("NC (Not Connected) is not allowed in logic equations")]
    BadNC,
    #[error("wrong number of pins on pin definition line - expected {expected}, found {found}")]
    BadPinCount { found: usize, expected: usize },
    #[error("expected pin definitions, found end of file")]
    BadPinEOF,
    #[error("expected plain pin name, found pin with suffix")]
    BadPinSuffix,
    #[error("use of VCC and GND is not allowed in equations")]
    BadPower,
    #[error("expected signature, found end of file")]
    BadSigEOF,
    #[error("unknown suffix found: '{suffix}'")]
    BadSuffix { suffix: String },
    #[error("expected {expected}, found other token")]
    BadToken { expected: &'static str },
    #[error("pin {pin} must be named {name}")]
    InvalidPowerPinName { pin: usize, name: &'static str },
    #[error(
        "pin {pin} cannot be named {name}, because the name is reserved for pin {expected_pin}"
    )]
    InvalidPowerPinLocation {
        pin: usize,
        name: &'static str,
        expected_pin: usize,
    },
    #[error(".{suffix} is not allowed when this type of GAL is used")]
    DisallowedControl { suffix: OutputSuffix },
    #[error("use of .{suffix} is only allowed for registered outputs")]
    InvalidControl { suffix: OutputSuffix },
    #[error("negation of {term} is not allowed")]
    InvertedSpecial { term: SpecialProductTerm },
    #[error("negation of .{suffix} is not allowed")]
    InvertedControl { suffix: OutputSuffix },
    #[error("{name} cannot be negated, use {hint} instead of /{name}")]
    InvertedPower {
        name: &'static str,
        hint: &'static str,
    },
    #[error("only one product term allowed (no OR)")]
    MoreThanOneProduct,
    #[error("missing clock definition (.CLK) of registered output")]
    NoCLK,
    #[error("'=' expected")]
    NoEquals,
    #[error("pin name expected after '/', found non-alphabetic character '{c}'")]
    NoPinName { c: char },
    #[error("pin name expected after '/', found end-of-line")]
    NoPinNameEOL,
    #[error(
        "pin {pin} is reserved for '{name}' on GAL20RA10 devices and can't be used in equations"
    )]
    ReservedInputGAL20RA10 { pin: usize, name: &'static str },
    #[error("pin {pin} is reserved for '{name}' in registered mode")]
    ReservedRegisteredInput { pin: usize, name: &'static str },
    #[error("pin {pin} can't be used as input in complex mode")]
    NotAnComplexModeInput { pin: usize },
    #[error("this pin can't be used as output")]
    NotAnOutput,
    #[error("{term} is defined twice")]
    RepeatedSpecial { term: SpecialProductTerm },
    #[error("multiple .{suffix} definitions for the same output")]
    RepeatedControl { suffix: OutputSuffix },
    #[error("output {name} is defined multiple times")]
    RepeatedOutput { name: String },
    #[error("pinname {name} is defined twice")]
    RepeatedPinName { name: String },
    #[error("the output must be defined to use .{suffix}")]
    UndefinedOutput { suffix: OutputSuffix },
    #[error("too many product terms in sum for pin (max: {max}, saw: {seen})")]
    TooManyProducts { max: usize, seen: usize },
    #[error("GAL16V8/20V8: tri. control for reg. output is not allowed")]
    TristateReg,
    #[error("unknown pinname '{name}'")]
    UnknownPin { name: String },
    #[error("tristate control without previous '.T'")]
    UnmatchedTristate,
}

// Adapt an ErrorCode to an Error.
pub fn at_line<Val>(line: LineNum, res: Result<Val, ErrorCode>) -> Result<Val, Error> {
    res.map_err(|e| Error { code: e, line })
}

#[derive(Debug, Clone, Copy)]
pub enum OutputSuffix {
    APRST,
    ARST,
    CLK,
    E,
}

impl fmt::Display for OutputSuffix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::APRST => "APRST",
            Self::ARST => "ARST",
            Self::CLK => "CLK",
            Self::E => "E",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SpecialProductTerm {
    AR,
    SP,
}

impl FromStr for SpecialProductTerm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "AR" => Self::AR,
            "SP" => Self::SP,
            _ => return Err(()),
        })
    }
}

impl fmt::Display for SpecialProductTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AR => "AR",
            Self::SP => "SP",
        })
    }
}
