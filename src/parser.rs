//
// parser.rs: Input parser
//
// Read the galasm-style input, and convert it to a 'Content'
// structure which feeds the rest of the pipeline. We check the special
// pin names meet the conventions, and the right number of pins are
// present, but try to leave other checks for later in the pipeline.
//

use chips::Chip;
use errors::Error;
use errors::ErrorCode;
use gal::Pin;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::iter::Peekable;
use std::rc::Rc;

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
    pub line_num: u32,
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

#[derive(Debug)]
enum Token {
    Item((NamedPin, Suffix)),
    Equals,
    And,
    Or,
}

#[derive(Debug)]
struct NamedPin {
    pub name: String,
    pub neg: bool,
}

////////////////////////////////////////////////////////////////////////
// Iterator with line number tracking.
//

struct LineTrackingIterator<I> {
    iter: I,
    // I can't think of a better way to keep access to this once this
    // iterator gets wrapped in others, than to use a RefCell.
    line_num_ref: Rc<RefCell<u32>>,
}

impl<I: Iterator> Iterator for LineTrackingIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let res = self.iter.next();

        if res.is_some() {
            *self.line_num_ref.borrow_mut() += 1;
        }

        res
    }
}

impl<I: Iterator> LineTrackingIterator<I> {
    fn new(iter: I) -> LineTrackingIterator<I> {
        LineTrackingIterator {
            iter: iter,
            line_num_ref: Rc::new(RefCell::new(0)),
        }
    }

    fn line_num(&self) -> LineNumber {
        LineNumber { line_num_ref: self.line_num_ref.clone() }
    }
}

struct LineNumber {
    line_num_ref: Rc<RefCell<u32>>,
}

impl LineNumber {
    fn get(&self) -> u32 {
        *self.line_num_ref.borrow()
    }
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
                    ()
                }
                _ => return Err(ErrorCode::BadChar),
            }
            None => return Ok(res),
        }
    }
}

// Tokenise a single pin name.
fn tokenise_pin<I>(chars: &mut Peekable<I>) -> Result<Token, ErrorCode>
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

    let named_pin = NamedPin { name: name, neg: neg };

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

fn remove_comment<'a>(s: &'a str) -> &'a str {
    match s.find(';') {
        Some(i) => &s[..i],
        None => s,
    }
}

pub fn parse_chip<'a, I>(line_iter: &mut I) -> Result<Chip, ErrorCode>
    where I: Iterator<Item = &'a str>
{
    match line_iter.next() {
        Some(name) => Chip::from_name(name.trim()),
        None => Err(ErrorCode::BadGALType),
    }
}

pub fn parse_signature<'a, I>(line_iter: &mut I) -> Result<Vec<u8>, ErrorCode>
    where I: Iterator<Item = &'a str>
{
    match line_iter.next() {
        Some(sig) => Ok(sig.bytes().take(8).collect::<Vec<u8>>()),
        None => Err(ErrorCode::BadEOF),
    }
}

pub fn parse_pins<'a, I>(chip: Chip, line_iter: &mut I) -> Result<Vec<(String, bool)>, ErrorCode>
    where I: Iterator<Item = &'a str>
{
    let mut pins = Vec::new();
    for _ in 0..2 {
        match line_iter.next() {
            Some(s) => {
                let tokens = tokenise(s)?;
                if tokens.len() != chip.num_pins() / 2 {
                    return Err(ErrorCode::BadPinCount);
                }
                for token in tokens.into_iter() {
                    match token {
                        Token::Item((name, suffix)) => {
                            if suffix == Suffix::None {
                                pins.push((name.name, name.neg));
                            } else {
                                return Err(ErrorCode::BadPin);
                            }
                        }
                        _ => return Err(ErrorCode::BadPin),
                    }
                }
            }
            None => return Err(ErrorCode::BadEOF),
        }
    }

    Ok(pins)
}

fn lookup_pin(chip: Chip, pin_map: &HashMap<String, Pin>, pin_name: &NamedPin) -> Result<Pin, ErrorCode> {
    let pin = pin_map.get(pin_name.name.as_str()).ok_or_else(|| {
        match pin_name.name.as_str() {
            "NC" => ErrorCode::BadNC,
            "AR" if chip == Chip::GAL22V10 => ErrorCode::BadARSP,
            "SP" if chip == Chip::GAL22V10 => ErrorCode::BadARSP,
            _ => ErrorCode::UnknownPin,
        }
    })?;

    Ok(Pin { pin: pin.pin, neg: pin.neg != pin_name.neg })
}

// Read a pin on the RHS (where suffices are not allowed), and convert to pin number.
fn parse_pin<I>(chip: Chip, pin_map: &HashMap<String, Pin>, iter: &mut I) -> Result<Pin, ErrorCode>
    where I: Iterator<Item=Token>
{
    let (named_pin, suffix) = match iter.next() {
        Some(Token::Item(item)) => item,
        _ => return Err(ErrorCode::BadToken),
    };

    if suffix != Suffix::None {
        return Err(ErrorCode::BadPin);
    }

    lookup_pin(chip, &pin_map, &named_pin)
}

// Parse and check the LHS (where suffices are allowed, but there are other constraints)
fn parse_lhs<I>(chip: Chip, pin_map: &HashMap<String, Pin>, iter: &mut I) -> Result<LHS, ErrorCode>
    where I: Iterator<Item=Token>
{
    Ok(match iter.next() {
        Some(Token::Item((named_pin, suffix))) => {
            if chip == Chip::GAL22V10 && (named_pin.name == "AR" || named_pin.name == "SP") {
                if suffix != Suffix::None {
                    return Err(ErrorCode::ARSPSuffix);
                }
                if named_pin.neg {
                    return Err(ErrorCode::InvertedARSP);
                }
                if named_pin.name == "AR" {
                    LHS::Ar
                } else {
                    LHS::Sp
                }
            } else {
                let pin = lookup_pin(chip, &pin_map, &named_pin)?;
                LHS::Pin((pin, suffix))
            }
        }
        _ => return Err(ErrorCode::BadToken),
    })
}

pub fn parse_equation(chip: Chip, pin_map: &HashMap<String, Pin>, line: &str, line_num: u32) -> Result<Equation, ErrorCode>
{
    let mut iter = tokenise(line)?.into_iter();

    let lhs = parse_lhs(chip, &pin_map, &mut iter)?;

    match iter.next() {
        Some(Token::Equals) => (),
        Some(_) => return Err(ErrorCode::NoEquals),
        None => return Err(ErrorCode::BadEOF),
    }

    let mut rhs = vec![parse_pin(chip, &pin_map, &mut iter)?];
    let mut is_or = vec![false];

    loop {
        match iter.next() {
            Some(Token::And) => {
                is_or.push(false);
                rhs.push(parse_pin(chip, &pin_map, &mut iter)?);
            }
            Some(Token::Or) => {
                is_or.push(true);
                rhs.push(parse_pin(chip, &pin_map, &mut iter)?);
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

fn build_pin_map(chip: Chip, pins: &Vec<(String, bool)>) -> Result<HashMap<String, Pin>, ErrorCode>
{
    let num_pins = chip.num_pins();
    let mut pin_map = HashMap::new();
    for ((name, neg), pin_num) in pins.clone().into_iter().zip(1..) {
        if pin_num == num_pins && (name.as_str(), neg) != ("VCC", false) {
            return Err(ErrorCode::BadVCC);
        }
        if pin_num == num_pins / 2 && (name.as_str(), neg) != ("GND", false) {
            return Err(ErrorCode::BadGND);
        }
        if name == "VCC" && pin_num != num_pins {
            return Err(ErrorCode::BadVCCLocation);
        }
        if name == "GND" && pin_num != num_pins / 2 {
            return Err(ErrorCode::BadGNDLocation);
        }
        if name != "NC" {
            if pin_map.contains_key(&name) {
                return Err(ErrorCode::RepeatedPinName);
            }

            if chip == Chip::GAL22V10 && (name == "AR" || name == "SP") {
                return Err(ErrorCode::ARSPAsPinName);
            }

            pin_map.insert(name, Pin { pin: pin_num, neg: neg });
        }
    }

    Ok(pin_map)
}

fn parse_core<'a, I>(mut line_iter: I, line_num: &LineNumber) -> Result<Content, ErrorCode>
    where I: Iterator<Item = &'a str>
{
    let chip = parse_chip(&mut line_iter)?;
    let signature = parse_signature(&mut line_iter)?;

    // After the first couple of lines we remove comments and
    // whitespace. Unlike galasm, we don't *require* a DESCRIPTION line,
    // but if we encounter one we stop there.
    let mut line_iter = line_iter
        .map(remove_comment)
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .take_while(|x| *x != "DESCRIPTION");

    let pins = parse_pins(chip, &mut line_iter)?;
    let pin_map = build_pin_map(chip, &pins)?;

    let equations = line_iter
        .map(|s| parse_equation(chip, &pin_map, s, line_num.get()))
        .collect::<Result<Vec<Equation>, ErrorCode>>()?;

    // The rest of the pipeline just wants string names.
    let pin_names = pins.iter().map(|(pin_name, neg)| {
        let mut full_name = if *neg { String::from("/") } else { String::new() };
        full_name.push_str(pin_name);
        full_name}).collect::<Vec<String>>();

    Ok(Content{
        chip: chip,
        sig: signature,
        pins: pin_names,
        eqns: equations,
    })
}

pub fn parse(file_name: &str) -> Result<Content, Error> {
    let data = fs::read_to_string(file_name).expect("Unable to read file");
    let line_iter = LineTrackingIterator::new(data.lines());
    let line_num = line_iter.line_num();
    parse_core(line_iter, &line_num).map_err(|e| Error { code: e, line: line_num.get() })
}
