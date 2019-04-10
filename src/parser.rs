use chips::Chip;
use errors::ErrorCode;

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
    pub eqns: Vec<Vec<Token>>,
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
    for _ in 0..1 {
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
    let equations = line_iter.map(tokenise).collect::<Result<Vec<Vec<Token>>, ErrorCode>>()?;

    Ok(Content{
        chip: gal_type,
        sig: signature,
        pins: pins,
        eqns: equations,
    })
}
