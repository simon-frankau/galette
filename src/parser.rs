use chips::Chip;
use errors::ErrorCode;
use gal::Pin;
use gal_builder;
use gal_builder::Equation;

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
    pin(Name),
    equals,
    and,
    or,
}

pub struct Content {
    pub chip: Chip,
    pub sig: Vec<u8>,
    pub pins: Vec<(String, bool)>,
    pub eqns: Vec<Equation>,
}

fn remove_comment<'a>(s: &'a str) -> &'a str {
    match s.find(';') {
        Some(i) => &s[..i],
        None => s,
    }
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
        _ => return Err(ErrorCode::NO_PIN_NAME),
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

    Ok(Token::pin(Name {
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
                    res.push(Token::equals);
                }
                '+' | '#' => {
                    chars.next();
                    res.push(Token::or);
                }
                '*' | '&' => {
                    chars.next();
                    res.push(Token::and);
                }
                '/' => res.push(build_pin(&mut chars)?),
                c if c.is_ascii_alphabetic() => res.push(build_pin(&mut chars)?),
                c if c.is_whitespace() => {
                    chars.next();
                    ()
                }
                _ => return Err(ErrorCode::BAD_CHAR),
            }
            None => return Ok(res),
        }
    }
}

pub fn parse_gal_type<'a, I>(line_iter: &mut I) -> Result<Chip, ErrorCode>
    where I: Iterator<Item = &'a str>
{
    match line_iter.next() {
        Some(name) => Chip::from_name(name),
        None => Err(ErrorCode::BAD_GAL_TYPE),
    }
}

pub fn parse_signature<'a, I>(line_iter: &mut I) -> Result<Vec<u8>, ErrorCode>
    where I: Iterator<Item = &'a str>
{
    match line_iter.next() {
        Some(sig) => Ok(sig.bytes().take(8).collect::<Vec<u8>>()),
        None => Err(ErrorCode::BAD_EOF),
    }
}

pub fn parse_pins<'a, I>(chip: Chip, line_iter: &mut I) -> Result<Vec<(String, bool)>, ErrorCode>
    where I: Iterator<Item = &'a str>
{
    let mut pins = Vec::new();
    for _ in 0..2 {
        match line_iter.next() {
            Some(line) => {
                let tokens = tokenise(line)?;
                if tokens.len() != chip.num_pins() / 2 {
                    return Err(ErrorCode::BAD_PIN_COUNT);
                }
                for token in tokens.into_iter() {
                    match token {
                        Token::pin(name) => {
                            if name.ext.is_none() {
                                pins.push((name.name, name.neg));
                            } else {
                                return Err(ErrorCode::BAD_PIN);
                            }
                        }
                        _ => return Err(ErrorCode::BAD_PIN),
                    }
                }
            }
            None => return Err(ErrorCode::BAD_EOF),
        }
    }

    // TODO: Sanity-check the pins? No extension, VCC and GND in the
    // right place, lack of repeats except NC, no AR/SP for GAL22V10,
    // etc.

    Ok(pins)
}

fn ext_to_suffix(s: &Option<String>) -> Result<i32, ErrorCode> {
   Ok(if let Some(s) = s {
       match s.as_str() {
           "T" => gal_builder::SUFFIX_T,
           "R" => gal_builder::SUFFIX_R,
           "E" => gal_builder::SUFFIX_E,
           "CLK" => gal_builder::SUFFIX_CLK,
           "APRST" => gal_builder::SUFFIX_APRST,
           "ARST" => gal_builder::SUFFIX_ARST,
           _ => return Err(ErrorCode::BAD_SUFFIX),
       }
   } else {
       gal_builder::SUFFIX_NON
   })
}

pub fn parse_equation(pin_map: &HashMap<String, (i32, bool)>, line: &str) -> Result<Equation, ErrorCode>
{
    let mut token_iter = tokenise(line)?.into_iter();

    // TODO: Suffix, line number, rhs, all the rest!
    let (lhs, suffix) = match token_iter.next() {
        Some(Token::pin(name)) => {
            let (pin_num, pin_neg) = pin_map.get(&name.name).ok_or(ErrorCode::UNKNOWN_PIN)?;
            let pin = Pin { pin: *pin_num as i8, neg: if name.neg != *pin_neg { 1 } else { 0 } };
            let suffix = ext_to_suffix(&name.ext)?;
            (pin, suffix)
        }
        _ => return Err(ErrorCode::BAD_TOKEN),
    };

    Ok(Equation {
        line_num: 0,
        lhs: lhs,
        suffix: suffix,
        num_rhs: 0,
        rhs: std::ptr::null(),
        ops: std::ptr::null(),
    })
}
/*
pub struct Equation {
    pub line_num: i32,
    pub lhs: Pin,
    pub suffix: i32,
    pub num_rhs: i32,
    pub rhs: *const Pin,
    pub ops: *const i8
}

*/

pub fn parse_stuff(file_name: &str) -> Result<Content, ErrorCode> {
    let data = fs::read_to_string(file_name).expect("Unable to read file");

    let mut line_iter = data.lines();
    let gal_type = parse_gal_type(&mut line_iter)?;
    let signature = parse_signature(&mut line_iter)?;

    // After the first couple of lines we remove comments etc.
    let mut line_iter = line_iter
        .map(remove_comment)
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .take_while(|x| *x != "DESCRIPTION");

    let pins = parse_pins(gal_type, &mut line_iter)?;
    let mut pin_map = (1..).zip(pins.clone().into_iter()).map(|(pin_num, (name, neg))| (name, (pin_num, neg))).collect::<HashMap<_, _>>();
    if gal_type == Chip::GAL22V10 {
        pin_map.insert(String::from("AR"), (24, false));
        pin_map.insert(String::from("SP"), (25, false));
    }

    let equations = line_iter.map(|line| parse_equation(&pin_map, line)).collect::<Result<Vec<Equation>, ErrorCode>>()?;

    Ok(Content{
        chip: gal_type,
        sig: signature,
        pins: pins,
        eqns: equations,
    })
}
