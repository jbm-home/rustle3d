//! Formulas of Seamly2D/Valentina files: arithmetic on numbers and named
//! values, as the files store it (`.` decimals, `;` between arguments,
//! English function names). Grammar and functions from the formats'
//! public description (spec 28): `||`, `&&`, comparisons, `+ -`, `* /`
//! and unary minus, `^` (right-associative, binding tighter than unary
//! minus: `-2^2 = -4`), `c ? a : b`; trigonometry in radians (`sin`) and
//! in degrees (`sinD`, `asinD`), `sqrt`, `abs`, `min`, `max`, `sum`,
//! `avg`, `fmod`, `rint`, `r2cm`, constants `_pi` and `_e`.

use super::ValError;

/// A parsed formula.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f64),
    Name(String),
    Neg(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Or,
    And,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Name(String),
    Op(&'static str),
}

const OPS: [&str; 19] = [
    "||", "&&", "<=", ">=", "!=", "==", "<", ">", "+", "-", "*", "/", "^", "(", ")", ";", "?", ":",
    ",",
];

fn is_name_char(c: char) -> bool {
    !c.is_whitespace() && !"+-*/^()<>=!&|?:;,".contains(c)
}

fn tokens(src: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let start = i;
            while chars
                .get(i)
                .is_some_and(|c| c.is_ascii_digit() || *c == '.')
            {
                i += 1;
            }
            if chars.get(i).is_some_and(|c| *c == 'e' || *c == 'E') {
                let mut j = i + 1;
                if chars.get(j).is_some_and(|c| *c == '+' || *c == '-') {
                    j += 1;
                }
                if chars.get(j).is_some_and(char::is_ascii_digit) {
                    i = j;
                    while chars.get(i).is_some_and(char::is_ascii_digit) {
                        i += 1;
                    }
                }
            }
            let text: String = chars.get(start..i).unwrap_or_default().iter().collect();
            let n = text
                .parse::<f64>()
                .map_err(|_| format!("bad number `{text}`"))?;
            out.push(Tok::Num(n));
            continue;
        }
        if let Some(op) = OPS.iter().find(|op| {
            op.chars()
                .enumerate()
                .all(|(k, oc)| chars.get(i + k) == Some(&oc))
        }) {
            out.push(Tok::Op(op));
            i += op.chars().count();
            continue;
        }
        if is_name_char(c) {
            let start = i;
            while chars.get(i).copied().is_some_and(is_name_char) {
                i += 1;
            }
            out.push(Tok::Name(
                chars.get(start..i).unwrap_or_default().iter().collect(),
            ));
            continue;
        }
        return Err(format!("unexpected `{c}`"));
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    at: usize,
}

impl Parser {
    fn peek_op(&self) -> Option<&'static str> {
        match self.toks.get(self.at) {
            Some(Tok::Op(op)) => Some(op),
            _ => None,
        }
    }

    fn eat(&mut self, op: &str) -> bool {
        if self.peek_op() == Some(op) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, op: &str) -> Result<(), String> {
        if self.eat(op) {
            Ok(())
        } else {
            Err(format!("`{op}` expected"))
        }
    }

    fn ternary(&mut self) -> Result<Expr, String> {
        let c = self.binary(0)?;
        if self.eat("?") {
            let a = self.ternary()?;
            self.expect(":")?;
            let b = self.ternary()?;
            return Ok(Expr::If(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    /// Binary operators by level, lowest first.
    fn binary(&mut self, level: usize) -> Result<Expr, String> {
        const LEVELS: [&[(&str, Op)]; 5] = [
            &[("||", Op::Or)],
            &[("&&", Op::And)],
            &[
                ("<=", Op::Le),
                (">=", Op::Ge),
                ("!=", Op::Ne),
                ("==", Op::Eq),
                ("<", Op::Lt),
                (">", Op::Gt),
            ],
            &[("+", Op::Add), ("-", Op::Sub)],
            &[("*", Op::Mul), ("/", Op::Div)],
        ];
        let Some(ops) = LEVELS.get(level) else {
            return self.unary();
        };
        let mut left = self.binary(level + 1)?;
        loop {
            let Some(op) = self
                .peek_op()
                .and_then(|t| ops.iter().find(|(s, _)| *s == t))
                .map(|(_, op)| *op)
            else {
                return Ok(left);
            };
            self.at += 1;
            let right = self.binary(level + 1)?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat("-") {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat("+") {
            return self.unary();
        }
        self.power()
    }

    fn power(&mut self) -> Result<Expr, String> {
        let base = self.primary()?;
        if self.eat("^") {
            let exp = self.unary()?;
            return Ok(Expr::Bin(Op::Pow, Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.toks.get(self.at).cloned() {
            Some(Tok::Num(n)) => {
                self.at += 1;
                Ok(Expr::Num(n))
            }
            Some(Tok::Name(name)) => {
                self.at += 1;
                if self.eat("(") {
                    let mut args = Vec::new();
                    if !self.eat(")") {
                        loop {
                            args.push(self.ternary()?);
                            if self.eat(")") {
                                break;
                            }
                            self.expect(";")?;
                        }
                    }
                    return Ok(Expr::Call(name, args));
                }
                Ok(Expr::Name(name))
            }
            Some(Tok::Op("(")) => {
                self.at += 1;
                let e = self.ternary()?;
                self.expect(")")?;
                Ok(e)
            }
            Some(Tok::Op(op)) => Err(format!("unexpected `{op}`")),
            None => Err("unexpected end".into()),
        }
    }
}

/// Parse a formula.
pub fn parse(src: &str) -> Result<Expr, ValError> {
    let err = |detail: String| ValError::Formula {
        formula: src.to_owned(),
        detail,
    };
    let toks = tokens(src).map_err(err)?;
    let mut p = Parser { toks, at: 0 };
    let e = p.ternary().map_err(err)?;
    if p.at < p.toks.len() {
        return Err(err("unexpected text after the formula".into()));
    }
    Ok(e)
}

/// Compute a formula; `names` gives the value of a name (`None`: not
/// known).
pub fn eval(src: &str, names: &dyn Fn(&str) -> Option<f64>) -> Result<f64, ValError> {
    let e = parse(src)?;
    let v = value(&e, names)?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(ValError::Formula {
            formula: src.to_owned(),
            detail: "the result is not a number".into(),
        })
    }
}

fn value(e: &Expr, names: &dyn Fn(&str) -> Option<f64>) -> Result<f64, ValError> {
    let b = |x: bool| if x { 1.0 } else { 0.0 };
    Ok(match e {
        Expr::Num(n) => *n,
        Expr::Name(n) => match n.as_str() {
            "_pi" => std::f64::consts::PI,
            "_e" => std::f64::consts::E,
            _ => names(n).ok_or_else(|| ValError::UnknownName { name: n.clone() })?,
        },
        Expr::Neg(a) => -value(a, names)?,
        Expr::If(c, a, z) => {
            if value(c, names)? != 0.0 {
                value(a, names)?
            } else {
                value(z, names)?
            }
        }
        Expr::Bin(op, l, r) => {
            let (x, y) = (value(l, names)?, value(r, names)?);
            match op {
                Op::Or => b(x != 0.0 || y != 0.0),
                Op::And => b(x != 0.0 && y != 0.0),
                Op::Lt => b(x < y),
                Op::Le => b(x <= y),
                Op::Gt => b(x > y),
                Op::Ge => b(x >= y),
                Op::Eq => b(x == y),
                Op::Ne => b(x != y),
                Op::Add => x + y,
                Op::Sub => x - y,
                Op::Mul => x * y,
                Op::Div => x / y,
                Op::Pow => x.powf(y),
            }
        }
        Expr::Call(f, args) => {
            let a: Vec<f64> = args
                .iter()
                .map(|e| value(e, names))
                .collect::<Result<_, _>>()?;
            call(f, &a)?
        }
    })
}

fn call(f: &str, a: &[f64]) -> Result<f64, ValError> {
    let bad = || ValError::Formula {
        formula: format!("{f}(…)"),
        detail: format!("`{f}` with {} arguments", a.len()),
    };
    let one = || match a {
        [x] => Ok(*x),
        _ => Err(bad()),
    };
    let two = || match a {
        [x, y] => Ok((*x, *y)),
        _ => Err(bad()),
    };
    let deg = f64::to_radians;
    Ok(match f {
        "sin" => one()?.sin(),
        "cos" => one()?.cos(),
        "tan" => one()?.tan(),
        "asin" => one()?.asin(),
        "acos" => one()?.acos(),
        "atan" => one()?.atan(),
        "atan2" => {
            let (y, x) = two()?;
            y.atan2(x)
        }
        "sinD" => deg(one()?).sin(),
        "cosD" => deg(one()?).cos(),
        "tanD" => deg(one()?).tan(),
        "asinD" => one()?.asin().to_degrees(),
        "acosD" => one()?.acos().to_degrees(),
        "atanD" => one()?.atan().to_degrees(),
        "degTorad" => deg(one()?),
        "radTodeg" => one()?.to_degrees(),
        "sinh" => one()?.sinh(),
        "cosh" => one()?.cosh(),
        "tanh" => one()?.tanh(),
        "asinh" => one()?.asinh(),
        "acosh" => one()?.acosh(),
        "atanh" => one()?.atanh(),
        "log2" => one()?.log2(),
        "log10" | "log" => one()?.log10(),
        "ln" => one()?.ln(),
        "exp" => one()?.exp(),
        "sqrt" => one()?.sqrt(),
        "sign" => {
            let x = one()?;
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            }
        }
        "rint" => (one()? + 0.5).floor(),
        "r2cm" => (one()? * 10.0 + 0.5).floor() / 10.0,
        "abs" => one()?.abs(),
        "fmod" => {
            let (x, y) = two()?;
            x % y
        }
        "sum" if !a.is_empty() => a.iter().sum(),
        "avg" if !a.is_empty() => a.iter().sum::<f64>() / a.len() as f64,
        "min" if !a.is_empty() => a.iter().copied().fold(f64::INFINITY, f64::min),
        "max" if !a.is_empty() => a.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        "sum" | "avg" | "min" | "max" => return Err(bad()),
        _ => {
            return Err(ValError::Formula {
                formula: format!("{f}(…)"),
                detail: format!("unknown function `{f}`"),
            });
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> f64 {
        eval(s, &|n| match n {
            "bust_circ" => Some(92.0),
            "#ease" => Some(2.0),
            "Line_A_A1" => Some(10.0),
            _ => None,
        })
        .unwrap()
    }

    #[test]
    fn precedence_and_power() {
        assert_eq!(ev("1+2*3"), 7.0);
        assert_eq!(ev("-2^2"), -4.0);
        assert_eq!(ev("2^3^2"), 512.0);
        assert_eq!(ev("(1+2)*3"), 9.0);
        assert_eq!(ev("2^-1"), 0.5);
        assert_eq!(ev("1.5e1"), 15.0);
    }

    #[test]
    fn names_and_functions() {
        assert_eq!(ev("bust_circ/4 + #ease"), 25.0);
        assert_eq!(ev("max(Line_A_A1; 3; 12)"), 12.0);
        assert!((ev("sinD(30)") - 0.5).abs() < 1e-12);
        assert!((ev("asinD(0.5)") - 30.0).abs() < 1e-9);
        assert_eq!(ev("bust_circ > 90 ? 1 : 2"), 1.0);
        assert_eq!(ev("r2cm(1.26)"), 1.3);
    }

    #[test]
    fn errors() {
        assert!(matches!(
            eval("waist_circ/2", &|_| None),
            Err(ValError::UnknownName { .. })
        ));
        assert!(matches!(
            eval("1+", &|_| None),
            Err(ValError::Formula { .. })
        ));
        assert!(matches!(
            eval("1/0", &|_| None),
            Err(ValError::Formula { .. })
        ));
        assert!(matches!(
            eval("foo(1)", &|_| None),
            Err(ValError::Formula { .. })
        ));
    }
}
