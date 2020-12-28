//
// errors.rs: Error codes
//
// Using error codes allows us to have a nice API, do
// internationalisation, etc. So, we define the error codes here. We
// have the error codes, and an error structure that combines the
// error code with the line number.
//

use thiserror::Error;

#[derive(Clone, Copy, Debug, Error)]
#[error("Error in line {}: {}", line, code)]
pub struct Error {
    pub code: ErrorCode,
    pub line: u32,
}

#[derive(Clone, Copy, Debug, Error)]
pub enum ErrorCode {
    #[error("GAL22V10: AR and SP is not allowed as pinname")]
    ARSPAsPinName,
    #[error("AR, SP: no suffix allowed")]
    ARSPSuffix,
    #[error("internal error: analyse_mode should never let you use this pin as an input")]
    BadAnalysis,
    #[error("use of AR and SP is not allowed in equations")]
    BadARSP,
    #[error("bad character in input")]
    BadChar,
    #[error("unexpected end of file")]
    BadEOF,
    #[error("unexpected end of line")]
    BadEOL,
    #[error("Line  1: type of GAL expected")]
    BadGALType,
    #[error("pin declaration: expected GND at GND pin")]
    BadGND,
    #[error("illegal VCC/GND assignment")]
    BadGNDLocation,
    #[error("NC (Not Connected) is not allowed in logic equations")]
    BadNC,
    #[error("illegal character in pin declaration")]
    BadPin,
    #[error("wrong number of pins")]
    BadPinCount,
    #[error("use of VCC and GND is not allowed in equations")]
    BadPower,
    #[error("unknown suffix found")]
    BadSuffix,
    #[error("unexpected token")]
    BadToken,
    #[error("pin declaration: expected VCC at VCC pin")]
    BadVCC,
    #[error("illegal VCC/GND assignment")]
    BadVCCLocation,
    #[error(".APRST is not allowed when this type of GAL is used")]
    DisallowedAPRST,
    #[error(".ARST is not allowed when this type of GAL is used")]
    DisallowedARST,
    #[error(".CLK is not allowed when this type of GAL is used")]
    DisallowedCLK,
    #[error("use of .CLK, .ARST, .APRST only allowed for registered outputs")]
    InvalidControl,
    #[error("negation of AR and SP is not allowed")]
    InvertedARSP,
    #[error(".E, .CLK, .ARST and .APRST is not allowed to be negated")]
    InvertedControl,
    #[error("use GND, VCC instead of /VCC, /GND")]
    InvertedPower,
    #[error("only one product term allowed (no OR)")]
    MoreThanOneProduct,
    #[error("missing clock definition (.CLK) of registered output")]
    NoCLK,
    #[error("'=' expected")]
    NoEquals,
    #[error("pinname expected after '/'")]
    NoPinName,
    #[error("GAL20RA10: pin 1 can't be used in equations")]
    NotAnInput1,
    #[error("mode 3: pins 1,11 are reserved for 'Clock' and '/OE'")]
    NotAnInput111,
    #[error("mode 3: pins 1,13 are reserved for 'Clock' and '/OE'")]
    NotAnInput113,
    #[error("mode 2: pins 12, 19 can't be used as input")]
    NotAnInput1219,
    #[error("GAL20RA10: pin 13 can't be used in equations")]
    NotAnInput13,
    #[error("mode 2: pins 15, 22 can't be used as input")]
    NotAnInput1522,
    #[error("this pin can't be used as output")]
    NotAnOutput,
    #[error("several .APRST definitions for the same output found")]
    RepeatedAPRST,
    #[error("AR or SP is defined twice")]
    RepeatedARSP,
    #[error("several .ARST definitions for the same output found")]
    RepeatedARST,
    #[error("several .CLK definitions for the same output found")]
    RepeatedCLK,
    #[error("same pin is defined multible as output")]
    RepeatedOutput,
    #[error("pinname defined twice")]
    RepeatedPinName,
    #[error("tristate control is defined twice")]
    RepeatedTristate,
    #[error("if using .APRST the output must be defined")]
    SoloAPRST,
    #[error("if using .ARST, the output must be defined")]
    SoloARST,
    #[error("if using .CLK, the output must be defined")]
    SoloCLK,
    #[error("if using .E, the output must be defined")]
    SoloEnable,
    #[error("too many product terms")]
    TooManyProducts,
    #[error("GAL16V8/20V8: tri. control for reg. output is not allowed")]
    TristateReg,
    #[error("unknown pinname")]
    UnknownPin,
    #[error("tristate control without previous '.T'")]
    UnmatchedTristate,
}

// Adapt an ErrorCode to an Error.
pub fn at_line<Val>(line: u32, res: Result<Val, ErrorCode>) -> Result<Val, Error> {
    res.map_err(|e| Error { code: e, line })
}
