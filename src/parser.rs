//
// parser.rs: Input parser
//
// Read the galasm-style input, and convert it to a 'Content'
// structure which feeds the rest of the pipeline. We check the special
// pin names meet the conventions, and the right number of pins are
// present, but try to leave other checks for later in the pipeline.
//

use std::{collections::HashMap, fs, iter::Peekable};

use crate::{
    chips::Chip,
    errors::{at_line, Error, ErrorCode},
    gal::Pin,
};

////////////////////////////////////////////////////////////////////////
// Parsing output
//

pub struct Content {
    pub chip: Chip,
    pub sig: Vec<u8>,
    pub pins: Vec<String>,
    pub eqns: Vec<Equation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Equation {
    pub line_num: usize,
    pub lhs: LHS,
    pub rhs: Vec<Pin>,
    pub is_or: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LHS {
    Pin((Pin, Suffix)),
    Ar,
    Sp,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Suffix {
    None,
    T,
    R,
    E,
    CLK,
    APRST,
    ARST,
}

////////////////////////////////////////////////////////////////////////
// Internal parsing structures
//

// Bit of a hack, since we can't get the line number once we've fallen
// off the end of the file. Use a special value that gets filled in
// correctly at the top level.
const EOF_LINE: usize = 0;

#[derive(Debug)]
enum Token {
    Item((NamedPin, Suffix)),
    Equals,
    And,
    Or,
}

#[derive(Debug)]
struct NamedPin {
    name: String,
    neg: bool,
}

////////////////////////////////////////////////////////////////////////
// Input tokenisation
//

// Tokenise a full line.
fn tokenise(s: &str) -> Result<Vec<Token>, ErrorCode> {
    let mut res = Vec::new();
    let mut chars = s.chars().peekable();
    loop {
        match chars.peek().cloned() {
            Some(c) => match c {
                '=' => {
                    chars.next();
                    res.push(Token::Equals);
                }
                '+' | '#' => {
                    chars.next();
                    res.push(Token::Or);
                }
                '*' | '&' => {
                    chars.next();
                    res.push(Token::And);
                }
                '/' => res.push(tokenise_pin(&mut chars)?),
                c if c.is_ascii_alphabetic() => res.push(tokenise_pin(&mut chars)?),
                c if c.is_whitespace() => {
                    chars.next();
                }
                _ => return Err(ErrorCode::BadChar),
            },
            None => return Ok(res),
        }
    }
}

// Tokenise a single pin name.
fn tokenise_pin<I>(chars: &mut Peekable<I>) -> Result<Token, ErrorCode>
where
    I: Iterator<Item = char>,
{
    let mut name = String::new();
    let mut neg = false;

    // Look for a negation prefix.
    if chars.peek() == Some(&'/') {
        chars.next();
        neg = true;
    }

    // First character must be alphabetic
    match chars.peek().cloned() {
        Some(c) if c.is_ascii_alphabetic() => {
            chars.next();
            name.push(c);
        }
        _ => return Err(ErrorCode::NoPinName),
    }

    // Body is alphanumeric
    loop {
        match chars.peek().cloned() {
            Some(c) if c.is_ascii_alphanumeric() => {
                chars.next();
                name.push(c);
            }
            _ => break,
        }
    }

    let named_pin = NamedPin { name, neg };

    // Look for extension
    let mut suffix = Suffix::None;
    if chars.peek().cloned() == Some('.') {
        chars.next();
        let mut ext = String::new();
        loop {
            match chars.peek().cloned() {
                Some(c) if c.is_ascii_alphanumeric() => {
                    chars.next();
                    ext.push(c);
                }
                _ => break,
            }
        }
        suffix = ext_to_suffix(&ext)?;
    }

    Ok(Token::Item((named_pin, suffix)))
}

fn ext_to_suffix(s: &str) -> Result<Suffix, ErrorCode> {
    Ok(match s {
        "T" => Suffix::T,
        "R" => Suffix::R,
        "E" => Suffix::E,
        "CLK" => Suffix::CLK,
        "APRST" => Suffix::APRST,
        "ARST" => Suffix::ARST,
        _ => return Err(ErrorCode::BadSuffix),
    })
}

////////////////////////////////////////////////////////////////////////
// Functions to extract specific elements.

fn remove_comment(s: &str) -> &str {
    match s.find(';') {
        Some(i) => &s[..i],
        None => s,
    }
}

fn next_or_fail<'a, I>(line_iter: &mut I, err_code: ErrorCode) -> Result<(usize, &'a str), Error>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    match line_iter.next() {
        Some(x) => Ok(x),
        None => err(EOF_LINE, err_code),
    }
}

fn parse_chip<'a, I>(line_iter: &mut I) -> Result<Chip, Error>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let (i, name) = next_or_fail(line_iter, ErrorCode::BadGALType)?;
    at_line(i, Chip::from_name(name.trim()))
}

fn parse_signature<'a, I>(line_iter: &mut I) -> Result<Vec<u8>, Error>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let (_, sig) = next_or_fail(line_iter, ErrorCode::BadEOF)?;
    Ok(sig.bytes().take(8).collect::<Vec<u8>>())
}

// Parse one line of pins
fn parse_pins<'a, I>(
    pin_map: &mut HashMap<String, Pin>,
    chip: Chip,
    row_num: usize,
    line_iter: &mut I,
) -> Result<Vec<(String, bool)>, Error>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut pins = Vec::new();
    let (line_num, line) = next_or_fail(line_iter, ErrorCode::BadEOF)?;
    let tokens = at_line(line_num, tokenise(line))?;
    let len = tokens.len();
    for token in tokens.into_iter() {
        match token {
            Token::Item((name, suffix)) if suffix == Suffix::None => {
                pins.push((name.name, name.neg))
            }
            Token::Item(_) => return err(line_num, ErrorCode::BadPin),
            _ => return err(line_num, ErrorCode::BadPin),
        }
    }

    // We test this afterwards in case there was a bad token
    // causing us to miscount. In that case, the earlier error
    // message willl be more useful.
    if len != chip.num_pins() / 2 {
        return err(line_num, ErrorCode::BadPinCount);
    }

    // Extend the pin map with the pins we've just defined.
    at_line(line_num, extend_pin_map(pin_map, chip, row_num, &pins))?;

    Ok(pins)
}

fn lookup_pin(
    chip: Chip,
    pin_map: &HashMap<String, Pin>,
    pin_name: &NamedPin,
) -> Result<Pin, ErrorCode> {
    let pin = pin_map
        .get(pin_name.name.as_str())
        .ok_or_else(|| match pin_name.name.as_str() {
            "NC" => ErrorCode::BadNC,
            "AR" if chip == Chip::GAL22V10 => ErrorCode::BadSpecial {
                term: pin_name.name.parse().unwrap(),
            },
            "SP" if chip == Chip::GAL22V10 => ErrorCode::BadSpecial {
                term: pin_name.name.parse().unwrap(),
            },
            _ => ErrorCode::UnknownPin,
        })?;

    Ok(Pin {
        pin: pin.pin,
        neg: pin.neg != pin_name.neg,
    })
}

// Read a pin on the RHS (where suffices are not allowed), and convert to pin number.
fn parse_pin<I>(chip: Chip, pin_map: &HashMap<String, Pin>, iter: &mut I) -> Result<Pin, ErrorCode>
where
    I: Iterator<Item = Token>,
{
    let (named_pin, suffix) = match iter.next() {
        Some(Token::Item(item)) => item,
        Some(_) => return Err(ErrorCode::BadToken),
        None => return Err(ErrorCode::BadEOL),
    };

    if suffix != Suffix::None {
        return Err(ErrorCode::BadPin);
    }

    lookup_pin(chip, pin_map, &named_pin)
}

// Parse and check the LHS (where suffices are allowed, but there are other constraints)
fn parse_lhs<I>(chip: Chip, pin_map: &HashMap<String, Pin>, iter: &mut I) -> Result<LHS, ErrorCode>
where
    I: Iterator<Item = Token>,
{
    Ok(match iter.next() {
        Some(Token::Item((named_pin, suffix))) => {
            if chip == Chip::GAL22V10 && (named_pin.name == "AR" || named_pin.name == "SP") {
                if suffix != Suffix::None {
                    return Err(ErrorCode::SpecialSuffix {
                        term: named_pin.name.parse().unwrap(),
                    });
                }
                if named_pin.neg {
                    return Err(ErrorCode::InvertedSpecial {
                        term: named_pin.name.parse().unwrap(),
                    });
                }

                if named_pin.name == "AR" {
                    LHS::Ar
                } else {
                    LHS::Sp
                }
            } else {
                let pin = lookup_pin(chip, pin_map, &named_pin)?;
                LHS::Pin((pin, suffix))
            }
        }
        _ => return Err(ErrorCode::BadToken),
    })
}

fn parse_equation<I>(
    chip: Chip,
    pin_map: &HashMap<String, Pin>,
    tokens: &mut I,
    line_num: usize,
) -> Result<Equation, ErrorCode>
where
    I: Iterator<Item = Token>,
{
    let lhs = parse_lhs(chip, pin_map, tokens)?;

    match tokens.next() {
        Some(Token::Equals) => (),
        Some(_) => return Err(ErrorCode::NoEquals),
        None => return Err(ErrorCode::BadEOF),
    }

    let mut rhs = vec![parse_pin(chip, pin_map, tokens)?];
    let mut is_or = vec![false];

    loop {
        match tokens.next() {
            Some(Token::And) => {
                is_or.push(false);
                rhs.push(parse_pin(chip, pin_map, tokens)?);
            }
            Some(Token::Or) => {
                is_or.push(true);
                rhs.push(parse_pin(chip, pin_map, tokens)?);
            }
            None => break,
            _ => return Err(ErrorCode::BadToken),
        }
    }

    Ok(Equation {
        line_num,
        lhs,
        rhs,
        is_or,
    })
}

// Add a row's worth of pins to the pin map.
fn extend_pin_map(
    pin_map: &mut HashMap<String, Pin>,
    chip: Chip,
    row_num: usize,
    pins: &[(String, bool)],
) -> Result<(), ErrorCode> {
    let num_pins = chip.num_pins();
    let first_pin = 1 + row_num * num_pins / 2;
    for ((name, neg), pin_num) in pins.iter().cloned().zip(first_pin..) {
        if pin_num == num_pins && (name.as_str(), neg) != ("VCC", false) {
            return Err(ErrorCode::InvalidPowerPinName {
                pin: pin_num,
                name: "VCC",
            });
        }
        if pin_num == num_pins / 2 && (name.as_str(), neg) != ("GND", false) {
            return Err(ErrorCode::InvalidPowerPinName {
                pin: pin_num,
                name: "GND",
            });
        }
        if name == "VCC" && pin_num != num_pins {
            return Err(ErrorCode::InvalidPowerPinLocation {
                pin: pin_num,
                name: "VCC",
                expected_pin: num_pins,
            });
        }
        if name == "GND" && pin_num != num_pins / 2 {
            return Err(ErrorCode::InvalidPowerPinLocation {
                pin: pin_num,
                name: "GND",
                expected_pin: num_pins / 2,
            });
        }
        if name != "NC" {
            if pin_map.contains_key(&name) {
                return Err(ErrorCode::RepeatedPinName { name });
            }

            if chip == Chip::GAL22V10 {
                // parse returns Ok if name is "AR" or "SP"
                if let Ok(term) = name.parse() {
                    return Err(ErrorCode::ReservedPinName { term });
                }
            }

            pin_map.insert(name, Pin { pin: pin_num, neg });
        }
    }

    Ok(())
}

fn parse_core<'a, I>(mut line_iter: I) -> Result<Content, Error>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let chip = parse_chip(&mut line_iter)?;
    let signature = parse_signature(&mut line_iter)?;

    // After the first couple of lines we remove comments and
    // whitespace. Unlike galasm, we don't *require* a DESCRIPTION line,
    // but if we encounter one we stop there.
    let mut line_iter = line_iter
        .map(|(i, x)| (i, str::trim(remove_comment(x))))
        .filter(|(_, x)| !x.is_empty())
        .take_while(|(_, x)| *x != "DESCRIPTION");

    // This is complicated because we want to process one line at a
    // time so that if there's an error it's reported on the
    // appropriate line of input.
    let mut pin_map = HashMap::new();
    let mut pins = parse_pins(&mut pin_map, chip, 0, &mut line_iter)?;
    let mut pins2 = parse_pins(&mut pin_map, chip, 1, &mut line_iter)?;
    pins.append(&mut pins2);

    // We tokenise the lines first, as the equation parser will want
    // to look ahead onto the token starting the next line (not yet
    // implemented).
    let mut equations = Vec::new();
    for (line_num, line) in line_iter {
	let tokens = at_line(line_num, tokenise(line))?;
        equations.push(at_line(
            line_num,
            parse_equation(chip, &pin_map, &mut tokens.into_iter(), line_num),
        )?);
    }

    // The rest of the pipeline just wants string names.
    let pin_names = pins
        .iter()
        .map(|(pin_name, neg)| {
            let mut full_name = if *neg {
                String::from("/")
            } else {
                String::new()
            };
            full_name.push_str(pin_name);
            full_name
        })
        .collect::<Vec<String>>();

    Ok(Content {
        chip,
        sig: signature,
        pins: pin_names,
        eqns: equations,
    })
}

fn err<T>(line_num: usize, error_code: ErrorCode) -> Result<T, Error> {
    Err(Error {
        code: error_code,
        line: line_num,
    })
}

pub fn parse(file_name: &str) -> Result<Content, Error> {
    let data = fs::read_to_string(file_name).expect("Unable to read file");
    parse_core((1..).zip(data.lines())).map_err(|e| {
        if e.line == EOF_LINE {
            Error {
                line: data.lines().count(),
                ..e
            }
        } else {
            e
        }
    })
}
