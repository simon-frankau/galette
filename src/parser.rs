use chips::Chip;
use errors::Error;
use errors::ErrorCode;
use gal::Pin;
use gal_builder::Suffix;

use std::collections::HashMap;
use std::fs;
use std::iter::Peekable;

#[derive(Debug)]
pub struct Name {
    pub name: String,
    pub neg: bool,
    pub ext: Option<String>,
}

#[derive(Debug)]
pub enum Token {
    PinName(Name),
    Equals,
    And,
    Or,
}

pub struct Content {
    pub chip: Chip,
    pub sig: Vec<u8>,
    pub pins: Vec<(String, bool)>,
    pub eqns: Vec<Equation>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PinOrArSp {
    Pin((Pin, Suffix)),
    Ar,
    Sp,
}

// Hack to own the memory
#[derive(Clone, Debug, PartialEq)]
pub struct Equation {
    pub line_num: u32,
    pub lhs: PinOrArSp,
    pub rhs: Vec<Pin>,
    pub is_or: Vec<bool>,
}

fn remove_comment<'a>((s, line): (&'a str, u32)) -> (&'a str, u32) {
    (match s.find(';') {
        Some(i) => &s[..i],
        None => s,
    }, line)
}

fn build_pin<I>(chars: &mut Peekable<I>) -> Result<Token, ErrorCode>
    where I: Iterator<Item = char>
{
    let mut name = String::new();
    let mut neg = false;

    // Look for a negation prefix.
    if chars.peek() == Some(&'/') {
        chars.next();
        neg = true;
    }

    // First character must be alphabetic
    match chars.peek().map(|x| *x) {
        Some(c) if c.is_ascii_alphabetic() => {
            chars.next();
            name.push(c);
        }
        _ => return Err(ErrorCode::NoPinName),
    }

    // Body is alphanumeric
    loop {
        match chars.peek().map(|x| *x) {
            Some(c) if c.is_ascii_alphanumeric() => {
                chars.next();
                name.push(c);
            }
            _ => break,
        }
    }

    // Look for extension
    let mut ext = None;
    if chars.peek().map(|x| *x) == Some('.') {
        chars.next();
        let mut ext_str = String::new();
        loop {
            match chars.peek().map(|x| *x) {
                Some(c) if c.is_ascii_alphanumeric() => {
                    chars.next();
                    ext_str.push(c);
                }
                _ => break,
            }
        }
        ext = Some(ext_str);
    }

    Ok(Token::PinName(Name {
        name: name,
        neg: neg,
        ext: ext,
    }))
}

fn tokenise(s: &str) -> Result<Vec<Token>, ErrorCode> {
    let mut res = Vec::new();
    let mut chars = s.chars().peekable();
    loop {
        match chars.peek().map(|x| *x) {
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
                '/' => res.push(build_pin(&mut chars)?),
                c if c.is_ascii_alphabetic() => res.push(build_pin(&mut chars)?),
                c if c.is_whitespace() => {
                    chars.next();
                    ()
                }
                _ => return Err(ErrorCode::BadChar),
            }
            None => return Ok(res),
        }
    }
}

fn at_line<Val>(line: u32, res: Result<Val, ErrorCode>) -> Result<Val, Error> {
   res.map_err(|e| Error { code: e, line: line })
}

pub fn parse_gal_type<'a, I>(line_iter: &mut I) -> Result<Chip, Error>
    where I: Iterator<Item = (&'a str, u32)>
{
    match line_iter.next() {
        Some((name, line)) => at_line(line, Chip::from_name(name)),
        None => Err(Error { code: ErrorCode::BadGALType, line: 0 }),
    }
}

pub fn parse_signature<'a, I>(line_iter: &mut I) -> Result<Vec<u8>, Error>
    where I: Iterator<Item = (&'a str, u32)>
{
    match line_iter.next() {
        Some((sig, _)) => Ok(sig.bytes().take(8).collect::<Vec<u8>>()),
        None => Err(Error { code: ErrorCode::BadEOF, line: 0 }),
    }
}

pub fn parse_pins<'a, I>(chip: Chip, line_iter: &mut I) -> Result<Vec<(String, bool)>, Error>
    where I: Iterator<Item = (&'a str, u32)>
{
    let mut pins = Vec::new();
    for _ in 0..2 {
        match line_iter.next() {
            Some((s, line)) => {
                let tokens = at_line(line, tokenise(s))?;
                if tokens.len() != chip.num_pins() / 2 {
                    return Err(Error { code: ErrorCode::BadPinCount, line: line });
                }
                for token in tokens.into_iter() {
                    match token {
                        Token::PinName(name) => {
                            if name.ext.is_none() {
                                pins.push((name.name, name.neg));
                            } else {
                                return Err(Error { code: ErrorCode::BadPin, line: line });
                            }
                        }
                        _ => return Err(Error { code: ErrorCode::BadPin, line: line }),
                    }
                }
            }
            None => return Err(Error { code: ErrorCode::BadEOF, line: 0 }),
        }
    }

    Ok(pins)
}

fn ext_to_suffix(s: &Option<String>) -> Result<Suffix, ErrorCode> {
   Ok(if let Some(s) = s {
       match s.as_str() {
           "T" => Suffix::T,
           "R" => Suffix::R,
           "E" => Suffix::E,
           "CLK" => Suffix::CLK,
           "APRST" => Suffix::APRST,
           "ARST" => Suffix::ARST,
           _ => return Err(ErrorCode::BadSuffix),
       }
   } else {
       Suffix::NONE
   })
}

fn lookup_pin(chip: Chip, pin_map: &HashMap<String, (u32, bool)>, pin_name: &str) -> Result<(u32, bool), ErrorCode> {
    pin_map.get(pin_name).map(|x| *x).ok_or_else(|| {
        match pin_name {
            "NC" => ErrorCode::BadNC,
            "AR" if chip == Chip::GAL22V10 => ErrorCode::BadARSP,
            "SP" if chip == Chip::GAL22V10 => ErrorCode::BadARSP,
            _ => ErrorCode::UnknownPin,
        }
    })
}

fn parse_pin<I>(chip: Chip, pin_map: &HashMap<String, (u32, bool)>, iter: &mut I) -> Result<Pin, ErrorCode>
    where I: Iterator<Item=Token>
{
    let pin = match iter.next() {
        Some(Token::PinName(pin)) => pin,
        _ => return Err(ErrorCode::BadToken),
    };

    if pin.ext.is_some() {
        return Err(ErrorCode::BadPin);
    }

    let (pin_num, pin_neg) = lookup_pin(chip, &pin_map, &pin.name)?;

    Ok(Pin {
        pin: pin_num,
        neg: pin.neg != pin_neg,
    })
}

pub fn parse_equation(chip: Chip, pin_map: &HashMap<String, (u32, bool)>, line: &str, line_num: u32) -> Result<Equation, ErrorCode>
{
    let mut token_iter = tokenise(line)?.into_iter();

    let lhs = match token_iter.next() {
        Some(Token::PinName(pin)) => {
            if chip == Chip::GAL22V10 && (pin.name == "AR" || pin.name == "SP") {
                if pin.ext.is_some() {
                    return Err(ErrorCode::ARSPSuffix);
                }
                if pin.neg {
                    return Err(ErrorCode::InvertedARSP);
                }
                if pin.name == "AR" {
                    PinOrArSp::Ar
                } else {
                    PinOrArSp::Sp
                }
            } else {
                let (pin_num, pin_neg) = lookup_pin(chip, &pin_map, &pin.name)?;
                let pin_def = Pin { pin: pin_num, neg: pin.neg != pin_neg };
                let suffix = ext_to_suffix(&pin.ext)?;
                PinOrArSp::Pin((pin_def, suffix))
            }
        }
        _ => return Err(ErrorCode::BadToken),
    };

    match token_iter.next() {
        Some(Token::Equals) => (),
        _ => return Err(ErrorCode::NoEquals),
    }

    let mut rhs = vec![parse_pin(chip, &pin_map, &mut token_iter)?];
    let mut is_or = vec![false];

    loop {
        match token_iter.next() {
            Some(Token::And) => {
                is_or.push(false);
                rhs.push(parse_pin(chip, &pin_map, &mut token_iter)?);
            }
            Some(Token::Or) => {
                is_or.push(true);
                rhs.push(parse_pin(chip, &pin_map, &mut token_iter)?);
            }
            None => break,
            _ => return Err(ErrorCode::BadToken),
       }
    }

    Ok(Equation {
        line_num: line_num,
        lhs: lhs,
        rhs: rhs,
        is_or: is_or,
    })
}

fn build_pin_map(gal_type: Chip, pins: &Vec<(String, bool)>) -> Result<HashMap<String, (u32, bool)>, ErrorCode>
{
    let num_pins = gal_type.num_pins();
    if pins[num_pins - 1] != (String::from("VCC"), false) {
        return Err(ErrorCode::BadVCCLocation);
    }
    if pins[num_pins/2 - 1] != (String::from("GND"), false) {
        return Err(ErrorCode::BadGNDLocation);
    }

    let mut pin_map = HashMap::new();
    for ((name, neg), pin_num) in pins.clone().into_iter().zip(1..) {
        if name != "NC" {
            if pin_map.contains_key(&name) {
                return Err(ErrorCode::RepeatedPinName);
            }

            if gal_type == Chip::GAL22V10 && (name =="AR" || name == "SP") {
                return Err(ErrorCode::ARSPAsPinName);
            }

            pin_map.insert(name, (pin_num, neg));
        }
    }

    Ok(pin_map)
}

pub fn parse_stuff(file_name: &str) -> Result<Content, Error> {
    let data = fs::read_to_string(file_name).expect("Unable to read file");

    let mut line_iter = data.lines().zip(1..);
    let gal_type = parse_gal_type(&mut line_iter)?;
    let signature = parse_signature(&mut line_iter)?;

    // After the first couple of lines we remove comments etc.
    let mut line_iter = line_iter
        .map(remove_comment)
        .map(|(s, i)| (s.trim(), i))
        .filter(|(x, _)| !x.is_empty())
        .take_while(|(x, _)| *x != "DESCRIPTION");

    let pins = parse_pins(gal_type, &mut line_iter)?;
    let pin_map = at_line(0, build_pin_map(gal_type, &pins))?;

    let equations = line_iter.map(|(s, line)| at_line(line, parse_equation(gal_type, &pin_map, s, line))).collect::<Result<Vec<Equation>, Error>>()?;

    Ok(Content{
        chip: gal_type,
        sig: signature,
        pins: pins,
        eqns: equations,
    })
}
