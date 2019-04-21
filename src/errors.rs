//
// errors.rs: Error codes
//
// Using error codes allows us to have a nice API, do
// internationalisation, etc. So, we define the error codes here. We
// have the error codes, and an error structure that combines the
// error code with the line number.
//

#[derive(Clone, Copy, Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub line: u32,
}

#[derive(Clone, Copy, Debug)]
pub enum ErrorCode {
    ARSPSuffix,
    ARSPAsPinName,
    BadARSP,
    BadChar,
    BadEOF,
    BadNC,
    BadPin,
    BadPinCount,
    BadPower,
    BadGALType,
    BadSuffix,
    BadToken,
    DisallowedCLK,
    DisallowedARST,
    DisallowedAPRST,
    InvalidControl,
    InvertedARSP,
    InvertedControl,
    InvertedPower,
    MoreThanOneProduct,
    NotAnInput,
    NotAnInput1,
    NotAnInput111,
    NotAnInput113,
    NotAnInput13,
    NotAnInput1219,
    NotAnInput1522,
    NotAnOutput,
    NoCLK,
    NoEquals,
    NoPinName,
    // TODO: I don't really believe in these.
    PrematureAPRST,
    PrematureARST,
    PrematureCLK,
    PrematureENABLE,
    RepeatedAPRST,
    RepeatedARST,
    RepeatedARSP,
    RepeatedCLK,
    RepeatedOutput,
    RepeatedPinName,
    RepeatedTristate,
    TooManyProducts,
    TristateReg,
    UnknownPin,
    UnmatchedTristate,
    BadVCCLocation,
    BadGNDLocation,
}

fn error_string(err_code: ErrorCode) -> &'static str {
    match err_code {
        ErrorCode::ARSPAsPinName => "GAL22V10: AR and SP is not allowed as pinname",
        ErrorCode::ARSPSuffix => "AR, SP: no suffix allowed",
        ErrorCode::BadARSP => "use of AR and SP is not allowed in equations",
        ErrorCode::BadNC => "NC (Not Connected) is not allowed in logic equations",
        ErrorCode::BadChar => "bad character in input",
        ErrorCode::BadEOF => "unexpected end of file",
        ErrorCode::BadGALType => "Line  1: type of GAL expected",
        ErrorCode::BadPin => "illegal character in pin declaration",
        ErrorCode::BadPinCount => "wrong number of pins",
        ErrorCode::BadPower => "use of VCC and GND is not allowed in equations",
        ErrorCode::BadSuffix => "unknown suffix found",
        ErrorCode::BadToken => "unexpected token",
        ErrorCode::InvertedARSP => "negation of AR and SP is not allowed",
        ErrorCode::InvalidControl => "use of .CLK, .ARST, .APRST only allowed for registered outputs",
        ErrorCode::InvertedControl => ".E, .CLK, .ARST and .APRST is not allowed to be negated",
        ErrorCode::InvertedPower => "use GND, VCC instead of /VCC, /GND",
        ErrorCode::MoreThanOneProduct => "only one product term allowed (no OR)",
        ErrorCode::NotAnInput => "pin not allowed in equations",
        ErrorCode::NotAnInput1 => "GAL20RA10: pin 1 can't be used in equations",
        ErrorCode::NotAnInput111 => "mode 3: pins 1,11 are reserved for 'Clock' and '/OE'",
        ErrorCode::NotAnInput113 => "mode 3: pins 1,13 are reserved for 'Clock' and '/OE'",
        ErrorCode::NotAnInput1219 => "mode 2: pins 12, 19 can't be used as input",
        ErrorCode::NotAnInput13 => "GAL20RA10: pin 13 can't be used in equations",
        ErrorCode::NotAnInput1522 => "mode 2: pins 15, 22 can't be used as input",
        ErrorCode::NotAnOutput => "this pin can't be used as output",
        ErrorCode::NoCLK => "missing clock definition (.CLK) of registered output",
        ErrorCode::NoPinName => "pinname expected after '/'",
        ErrorCode::NoEquals => "'=' expected",
        ErrorCode::PrematureAPRST => "before using .APRST the output must be defined",
        ErrorCode::PrematureARST => "before using .ARST, the output must be defined",
        ErrorCode::PrematureCLK => "before using .CLK, the output must be defined",
        ErrorCode::PrematureENABLE => "before using .E, the output must be defined",
        ErrorCode::RepeatedAPRST => "several .APRST definitions for the same output found",
        ErrorCode::RepeatedARST => "several .ARST definitions for the same output found",
        ErrorCode::RepeatedARSP => "AR or SP is defined twice",
        ErrorCode::RepeatedCLK => "several .CLK definitions for the same output found",
        ErrorCode::RepeatedOutput => "same pin is defined multible as output",
        ErrorCode::RepeatedPinName => "pinname defined twice",
        ErrorCode::RepeatedTristate => "tristate control is defined twice",
        ErrorCode::TooManyProducts => "too many product terms",
        ErrorCode::TristateReg => "GAL16V8/20V8: tri. control for reg. output is not allowed",
        ErrorCode::UnknownPin => "unknown pinname",
        ErrorCode::UnmatchedTristate => "tristate control without previous '.T'",
        ErrorCode::BadVCCLocation => "pin declaration: expected VCC at VCC pin",
        ErrorCode::BadGNDLocation => "pin declaration: expected GND at GND pin",
        ErrorCode::DisallowedCLK => ".CLK is not allowed when this type of GAL is used",
        ErrorCode::DisallowedARST => ".ARST is not allowed when this type of GAL is used",
        ErrorCode::DisallowedAPRST => ".APRST is not allowed when this type of GAL is used",
    }
}

// Adapt an ErrorCode to an Error.
pub fn at_line<Val>(line: u32, res: Result<Val, ErrorCode>) -> Result<Val, Error> {
   res.map_err(|e| Error { code: e, line: line })
}

pub fn print_error(err: Error) {
    println!("Error in line {}: {}", err.line, error_string(err.code));
}
