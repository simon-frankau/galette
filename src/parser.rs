//
// parser.rs: Input parser
//
// Read the galasm-style input, and convert it to a 'Content'
// structure which feeds the rest of the pipeline.
//

use chips::Chip;
use errors::at_line;
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
    pub pins: Vec<(String, bool)>,
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
    line_num: Rc<RefCell<u32>>,
}


impl<I: Iterator> Iterator for LineTrackingIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let res = self.iter.next();

        if res.is_some() {
            *self.line_num.borrow_mut() += 1;
        }

        res
    }
}

impl<I: Iterator> LineTrackingIterator<I> {
    fn new(iter: I) -> LineTrackingIterator<I> {
        LineTrackingIterator {
            iter: iter,
            line_num: Rc::new(RefCell::new(0)),
        }
    }

    fn line_num(&self) -> Rc<RefCell<u32>> {
        self.line_num.clone()
    }
}

////////////////////////////////////////////////////////////////////////
// 
//

fn remove_comment<'a>((s, line): (&'a str, u32)) -> (&'a str, u32) {
    (match s.find(';') {
        Some(i) => &s[..i],
        None => s,
    }, line)
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

    let named_pin = NamedPin { name: name, neg: neg };

    // Look for extension
    let mut suffix = Suffix::None;
    if chars.peek().map(|x| *x) == Some('.') {
        chars.next();
        let mut ext = String::new();
        loop {
            match chars.peek().map(|x| *x) {
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
                        Token::Item((name, suffix)) => {
                            if suffix == Suffix::None {
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

fn lookup_pin(chip: Chip, pin_map: &HashMap<String, Pin>, pin_name: &NamedPin) -> Result<Pin, ErrorCode> {
    let pin = pin_map.get(pin_name.name.as_str()).map(|x| *x).ok_or_else(|| {
        match pin_name.name.as_str() {
            "NC" => ErrorCode::BadNC,
            "AR" if chip == Chip::GAL22V10 => ErrorCode::BadARSP,
            "SP" if chip == Chip::GAL22V10 => ErrorCode::BadARSP,
            _ => ErrorCode::UnknownPin,
        }
    })?;

    Ok(Pin { pin: pin.pin, neg: pin.neg != pin_name.neg })
}

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

pub fn parse_equation(chip: Chip, pin_map: &HashMap<String, Pin>, line: &str, line_num: u32) -> Result<Equation, ErrorCode>
{
    let mut token_iter = tokenise(line)?.into_iter();

    let lhs = match token_iter.next() {
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

fn build_pin_map(gal_type: Chip, pins: &Vec<(String, bool)>) -> Result<HashMap<String, Pin>, ErrorCode>
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

            pin_map.insert(name, Pin { pin: pin_num, neg: neg });
        }
    }

    Ok(pin_map)
}

pub fn parse_stuff(file_name: &str) -> Result<Content, Error> {
    let data = fs::read_to_string(file_name).expect("Unable to read file");

    let mut line_iter = LineTrackingIterator::new(data.lines().zip(1..));
    let line_num_ref = line_iter.line_num();
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

    let equations = line_iter.map(|(s, line)| at_line(line, parse_equation(gal_type, &pin_map, s, line))).collect::<Result<Vec<Equation>, Error>>();

    let equations = match equations {
        Ok(e) => e,
        Err(err) => {
            return at_line(*line_num_ref.borrow(), Err(err.code));
        }
    };

    Ok(Content{
        chip: gal_type,
        sig: signature,
        pins: pins,
        eqns: equations,
    })
}
