//! Formulas for dimensions and named variables: `waist / 4 + 10 mm`.
//!
//! A formula is arithmetic (`+ - * /`, parentheses, `min`, `max`, `abs`,
//! `sqrt`) on numbers, the body's measurements (`height`, `bust`, `waist`,
//! `hips`, and those of [`Measure`]: `neck`, `arm_length`…) and the
//! project's named variables and dimensions. Every value carries its
//! length dimension (mm¹ for a length, mm⁰ for a plain number or an angle
//! in degrees), checked as it is computed, so `waist + 2` is refused
//! (`formula.unit`) while `waist + 2 cm` is a length. Numbers take a unit:
//! `mm`, `cm`, `m`, `in`, or `deg` (a plain number). Lengths are computed
//! in millimetres.

use std::collections::BTreeMap;

use crate::body::Measure;
use crate::id::ConstraintId;
use crate::message::{Localizable, Message};
use crate::model::{Avatar, Constraint};

/// The four measurements a body is made from, as formulas name them.
pub const BODY_NAMES: [&str; 4] = ["height", "bust", "waist", "hips"];

/// Whether `name` is a body measurement in formulas: one of the four or
/// of [`Measure`].
pub fn is_body_name(name: &str) -> bool {
    BODY_NAMES.contains(&name) || Measure::from_name(name).is_some()
}

/// `text` with every name `from` written `to` (other names, numbers and
/// units as they were).
pub fn rename(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        out.push_str(if word == from { to } else { word });
        word.clear();
    };
    for c in text.chars() {
        let starts = c.is_ascii_alphabetic() || c == '_';
        let continues = !word.is_empty() && c.is_ascii_digit();
        if starts || continues {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// Why a formula cannot be computed.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum FormulaError {
    #[error("the formula cannot be read at character {at}")]
    Syntax { at: usize },
    #[error("`{name}` is neither a body measurement nor a variable")]
    UnknownName { name: String },
    #[error("`{name}` needs a body: set the measurements first")]
    NoBody { name: String },
    #[error("`{name}` is computed from itself")]
    Cycle { name: String },
    #[error("the units do not match: {found} where {expected} is needed")]
    Unit {
        expected: &'static str,
        found: &'static str,
    },
    #[error("the formula has no value (a division by zero, the root of a negative)")]
    Domain,
    #[error("the formula gives {value}, out of the dimension's range")]
    Range { value: String },
    #[error(
        "`{name}` cannot name a variable (a letter or _, then letters, digits or _; not a body measurement or a function)"
    )]
    BadName { name: String },
}

impl Localizable for FormulaError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Syntax { at } => Message::new("formula.syntax", text).arg("at", at),
            Self::UnknownName { name } => {
                Message::new("formula.unknown_name", text).arg("name", name)
            }
            Self::NoBody { name } => Message::new("formula.no_body", text).arg("name", name),
            Self::Cycle { name } => Message::new("formula.cycle", text).arg("name", name),
            Self::Unit { expected, found } => Message::new("formula.unit", text)
                .arg("expected", expected)
                .arg("found", found),
            Self::Domain => Message::new("formula.domain", text),
            Self::Range { value } => Message::new("formula.range", text).arg("value", value),
            Self::BadName { name } => Message::new("formula.bad_name", text).arg("name", name),
        }
    }
}

/// A computed value: a number and its length dimension (mm to that
/// power).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quantity {
    pub value: f64,
    pub power: i32,
}

/// What a dimension's power is called in messages.
pub fn unit_name(power: i32) -> &'static str {
    match power {
        0 => "a plain number",
        1 => "a length",
        2 => "an area",
        _ => "another unit",
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Number(f64, i32),
    Name(String),
    Neg(Box<Node>),
    Bin(char, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
}

const FUNCTIONS: [&str; 4] = ["min", "max", "abs", "sqrt"];

struct Parser<'a> {
    text: &'a str,
    at: usize,
}

impl Parser<'_> {
    fn skip(&mut self) {
        while self.text[self.at..].starts_with(char::is_whitespace) {
            self.at += self.text[self.at..]
                .chars()
                .next()
                .map_or(1, char::len_utf8);
        }
    }

    fn error(&self) -> FormulaError {
        FormulaError::Syntax {
            at: self.text[..self.at].chars().count() + 1,
        }
    }

    fn eat(&mut self, c: char) -> bool {
        self.skip();
        if self.text[self.at..].starts_with(c) {
            self.at += c.len_utf8();
            true
        } else {
            false
        }
    }

    fn word(&mut self) -> Option<String> {
        self.skip();
        let rest = &self.text[self.at..];
        let first = rest.chars().next()?;
        if !(first.is_ascii_alphabetic() || first == '_') {
            return None;
        }
        let len = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        self.at += len;
        Some(rest[..len].to_owned())
    }

    fn sum(&mut self) -> Result<Node, FormulaError> {
        let mut left = self.product()?;
        loop {
            let op = if self.eat('+') {
                '+'
            } else if self.eat('-') {
                '-'
            } else {
                return Ok(left);
            };
            left = Node::Bin(op, Box::new(left), Box::new(self.product()?));
        }
    }

    fn product(&mut self) -> Result<Node, FormulaError> {
        let mut left = self.unary()?;
        loop {
            let op = if self.eat('*') {
                '*'
            } else if self.eat('/') {
                '/'
            } else {
                return Ok(left);
            };
            left = Node::Bin(op, Box::new(left), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Node, FormulaError> {
        if self.eat('-') {
            return Ok(Node::Neg(Box::new(self.unary()?)));
        }
        if self.eat('+') {
            return self.unary();
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<Node, FormulaError> {
        if self.eat('(') {
            let inner = self.sum()?;
            return if self.eat(')') {
                Ok(inner)
            } else {
                Err(self.error())
            };
        }
        self.skip();
        let rest = &self.text[self.at..];
        let digits = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(rest.len());
        if digits > 0 {
            let value: f64 = rest[..digits].parse().map_err(|_| self.error())?;
            self.at += digits;
            let save = self.at;
            let (scale, power) = match self.word().as_deref() {
                Some("mm") => (1.0, 1),
                Some("cm") => (10.0, 1),
                Some("m") => (1000.0, 1),
                Some("in") => (25.4, 1),
                Some("deg") => (1.0, 0),
                Some(_) => return Err(self.error()),
                None => {
                    // A plain number, or degrees written with their sign.
                    self.at = save;
                    self.eat('°');
                    (1.0, 0)
                }
            };
            return Ok(Node::Number(value * scale, power));
        }
        let Some(name) = self.word() else {
            return Err(self.error());
        };
        if self.eat('(') {
            if !FUNCTIONS.contains(&name.as_str()) {
                return Err(FormulaError::UnknownName { name });
            }
            let mut args = vec![self.sum()?];
            while self.eat(',') {
                args.push(self.sum()?);
            }
            if !self.eat(')') {
                return Err(self.error());
            }
            return Ok(Node::Call(name, args));
        }
        Ok(Node::Name(name))
    }
}

fn parse(text: &str) -> Result<Node, FormulaError> {
    let mut p = Parser { text, at: 0 };
    let node = p.sum()?;
    p.skip();
    if p.at < text.len() {
        return Err(p.error());
    }
    Ok(node)
}

/// Whether `name` may name a variable.
pub fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !is_body_name(name)
        && !FUNCTIONS.contains(&name)
        && !["mm", "cm", "m", "in", "deg"].contains(&name)
}

/// What formulas are computed with: a body (the avatar's measurements,
/// or none), the named variables' formulas and the named dimensions
/// (their formula, else their value).
#[derive(Clone, Copy, Debug)]
pub struct Scope<'a> {
    pub body: Option<&'a Avatar>,
    pub variables: &'a BTreeMap<String, String>,
    pub dimensions: &'a BTreeMap<ConstraintId, Constraint>,
}

impl<'a> Scope<'a> {
    /// Compute `text`.
    pub fn eval(&self, text: &str) -> Result<Quantity, FormulaError> {
        self.eval_in(text, &mut Vec::new())
    }

    fn eval_in(&self, text: &str, stack: &mut Vec<String>) -> Result<Quantity, FormulaError> {
        let node = parse(text)?;
        self.node(&node, stack)
    }

    fn name(&self, name: &str, stack: &mut Vec<String>) -> Result<Quantity, FormulaError> {
        if is_body_name(name) {
            let avatar = self.body.ok_or_else(|| FormulaError::NoBody {
                name: name.to_owned(),
            })?;
            let b = &avatar.body;
            let cm = match (name, Measure::from_name(name)) {
                ("height", _) => b.height_cm,
                ("bust", _) => b.bust_cm,
                ("waist", _) => b.waist_cm,
                ("hips", _) => b.hips_cm,
                (_, Some(m)) => avatar.measure(m).cm,
                (_, None) => 0.0,
            };
            return Ok(Quantity {
                value: cm * 10.0,
                power: 1,
            });
        }
        let formula = match self.variables.get(name) {
            Some(f) => f,
            None => {
                let c = self
                    .dimension(name)
                    .ok_or_else(|| FormulaError::UnknownName {
                        name: name.to_owned(),
                    })?;
                match (c.formula(), c.value(), c.value_power()) {
                    (Some(f), ..) => f,
                    (None, Some(value), Some(power)) => return Ok(Quantity { value, power }),
                    _ => {
                        return Err(FormulaError::UnknownName {
                            name: name.to_owned(),
                        });
                    }
                }
            }
        };
        if stack.iter().any(|n| n == name) {
            return Err(FormulaError::Cycle {
                name: name.to_owned(),
            });
        }
        stack.push(name.to_owned());
        let q = self.eval_in(formula, stack);
        stack.pop();
        q
    }

    fn node(&self, node: &Node, stack: &mut Vec<String>) -> Result<Quantity, FormulaError> {
        let same = |a: Quantity, b: Quantity| {
            if a.power == b.power {
                Ok(())
            } else {
                Err(FormulaError::Unit {
                    expected: unit_name(a.power),
                    found: unit_name(b.power),
                })
            }
        };
        let q = match node {
            Node::Number(value, power) => Quantity {
                value: *value,
                power: *power,
            },
            Node::Name(name) => self.name(name, stack)?,
            Node::Neg(inner) => {
                let q = self.node(inner, stack)?;
                Quantity {
                    value: -q.value,
                    ..q
                }
            }
            Node::Bin(op, a, b) => {
                let (a, b) = (self.node(a, stack)?, self.node(b, stack)?);
                match op {
                    '+' | '-' => {
                        same(a, b)?;
                        let value = if *op == '+' {
                            a.value + b.value
                        } else {
                            a.value - b.value
                        };
                        Quantity { value, ..a }
                    }
                    '*' => Quantity {
                        value: a.value * b.value,
                        power: a.power + b.power,
                    },
                    _ => {
                        if b.value == 0.0 {
                            return Err(FormulaError::Domain);
                        }
                        Quantity {
                            value: a.value / b.value,
                            power: a.power - b.power,
                        }
                    }
                }
            }
            Node::Call(f, args) => {
                let args: Vec<Quantity> = args
                    .iter()
                    .map(|a| self.node(a, stack))
                    .collect::<Result<_, _>>()?;
                match (f.as_str(), args.as_slice()) {
                    ("min" | "max", [first, rest @ ..]) if !rest.is_empty() => {
                        let mut out = *first;
                        for q in rest {
                            same(*first, *q)?;
                            out.value = if f == "min" {
                                out.value.min(q.value)
                            } else {
                                out.value.max(q.value)
                            };
                        }
                        out
                    }
                    ("abs", [q]) => Quantity {
                        value: q.value.abs(),
                        ..*q
                    },
                    ("sqrt", [q]) => {
                        if q.value < 0.0 || q.power % 2 != 0 {
                            return Err(if q.value < 0.0 {
                                FormulaError::Domain
                            } else {
                                FormulaError::Unit {
                                    expected: "an area or a plain number",
                                    found: unit_name(q.power),
                                }
                            });
                        }
                        Quantity {
                            value: q.value.sqrt(),
                            power: q.power / 2,
                        }
                    }
                    _ => return Err(FormulaError::Syntax { at: 1 }),
                }
            }
        };
        if q.value.is_finite() {
            Ok(q)
        } else {
            Err(FormulaError::Domain)
        }
    }

    /// The dimension named `name`, if any.
    pub fn dimension(&self, name: &str) -> Option<&'a Constraint> {
        self.dimensions.values().find(|c| c.name() == Some(name))
    }

    /// Compute `text` as a value of length dimension `power`: a length in
    /// mm (1) or a plain number (0, an angle in degrees).
    pub fn eval_as(&self, text: &str, power: i32) -> Result<f64, FormulaError> {
        let q = self.eval(text)?;
        if q.power != power {
            return Err(FormulaError::Unit {
                expected: unit_name(power),
                found: unit_name(q.power),
            });
        }
        Ok(q.value)
    }

    /// Whether `text` depends on the body, directly or through variables
    /// (a formula that does not grades nothing).
    pub fn uses_body(&self, text: &str) -> bool {
        fn walk(scope: &Scope<'_>, node: &Node, seen: &mut Vec<String>) -> bool {
            match node {
                Node::Number(..) => false,
                Node::Name(n) if is_body_name(n) => true,
                Node::Name(n) => {
                    if seen.contains(n) {
                        return false;
                    }
                    seen.push(n.clone());
                    scope
                        .variables
                        .get(n)
                        .map(String::as_str)
                        .or_else(|| scope.dimension(n).and_then(Constraint::formula))
                        .and_then(|f| parse(f).ok())
                        .is_some_and(|inner| walk(scope, &inner, seen))
                }
                Node::Neg(a) => walk(scope, a, seen),
                Node::Bin(_, a, b) => walk(scope, a, seen) || walk(scope, b, seen),
                Node::Call(_, args) => args.iter().any(|a| walk(scope, a, seen)),
            }
        }
        parse(text).is_ok_and(|n| walk(self, &n, &mut Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> Avatar {
        Avatar {
            body: crate::Body {
                masculinity: 0.0,
                height_cm: 168.0,
                bust_cm: 90.0,
                waist_cm: 72.0,
                hips_cm: 98.0,
                age_years: None,
            },
            preset: None,
            mesh: None,
            measures: crate::Measures::from([(Measure::Neck, 36.5)]),
        }
    }

    #[test]
    fn computes_with_units_and_the_body() {
        let b = body();
        let vars = BTreeMap::from([
            ("ease".to_owned(), "2 cm".to_owned()),
            ("quarter".to_owned(), "waist / 4 + ease".to_owned()),
        ]);
        let s = Scope {
            body: Some(&b),
            variables: &vars,
            dimensions: &BTreeMap::new(),
        };
        assert_eq!(s.eval_as("waist / 4 + 10 mm", 1), Ok(190.0));
        assert_eq!(s.eval_as("quarter", 1), Ok(200.0));
        assert_eq!(s.eval_as("max(bust, hips) / 2 - 1 in", 1), Ok(490.0 - 25.4));
        assert_eq!(s.eval_as("90 deg - 3", 0), Ok(87.0));
        assert_eq!(s.eval_as("sqrt(bust * hips) * 0 + 3 mm", 1), Ok(3.0));
        assert_eq!(s.eval_as("-(2 cm) * 2", 1), Ok(-40.0));
        assert!(s.uses_body("quarter"));
        assert!(!s.uses_body("ease * 2"));
        // The other measurements: typed, or estimated.
        assert_eq!(s.eval_as("neck / 2", 1), Ok(182.5));
        let knee = Measure::Knee.estimate(&b.body) * 10.0;
        assert_eq!(s.eval_as("knee", 1), Ok(knee));
        assert!(s.uses_body("arm_length - 2 cm"));
    }

    #[test]
    fn renames_whole_names_only() {
        assert_eq!(
            rename("neck/2 + neckline + 10mm - neck", "neck", "my_neck"),
            "my_neck/2 + neckline + 10mm - my_neck"
        );
    }

    #[test]
    fn refuses_what_does_not_compute() {
        let b = body();
        let vars = BTreeMap::from([
            ("a".to_owned(), "b + 1 mm".to_owned()),
            ("b".to_owned(), "a".to_owned()),
        ]);
        let s = Scope {
            body: Some(&b),
            variables: &vars,
            dimensions: &BTreeMap::new(),
        };
        let code = |t: &str, p: i32| s.eval_as(t, p).map_err(|e| e.message().code);
        assert_eq!(code("waist + 2", 1), Err("formula.unit"));
        assert_eq!(code("waist", 0), Err("formula.unit"));
        assert_eq!(code("collar / 2", 1), Err("formula.unknown_name"));
        assert_eq!(code("a", 1), Err("formula.cycle"));
        assert_eq!(code("waist / 0", 1), Err("formula.domain"));
        assert_eq!(code("waist / 4 +", 1), Err("formula.syntax"));
        assert_eq!(code("3 furlongs", 1), Err("formula.syntax"));
        assert_eq!(code("(waist", 1), Err("formula.syntax"));
        let none = Scope {
            body: None,
            variables: &vars,
            dimensions: &BTreeMap::new(),
        };
        assert_eq!(
            none.eval_as("hips", 1).map_err(|e| e.message().code),
            Err("formula.no_body")
        );
        assert!(valid_name("ease_2"));
        assert!(!valid_name("waist") && !valid_name("2x") && !valid_name("min"));
        assert!(!valid_name("arm_length"));
    }
}
