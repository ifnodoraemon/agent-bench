//! A small, safe arithmetic expression evaluator for the `calculator` tool.
//!
//! The old tool interpolated model output into `python3 -c "print(...)"`,
//! which is arbitrary code execution. This parser accepts only numbers,
//! operators, parentheses and a fixed function/constant allowlist.

use anyhow::{bail, Result};

const MAX_LEN: usize = 512;
const MAX_DEPTH: usize = 64;
const MAX_OPS: usize = 10_000;

pub fn evaluate(expression: &str) -> Result<f64> {
    if expression.len() > MAX_LEN {
        bail!("Expression is too long (max {MAX_LEN} bytes)");
    }
    let mut parser = Parser {
        input: expression.as_bytes(),
        pos: 0,
        depth: 0,
        ops: 0,
    };
    let value = parser.parse_expr()?;
    parser.skip_ws();
    if parser.pos != parser.input.len() {
        bail!("Unexpected trailing input at byte {}", parser.pos);
    }
    if !value.is_finite() {
        bail!("Expression did not produce a finite number");
    }
    Ok(value)
}

pub fn evaluate_to_string(expression: &str) -> Result<String> {
    Ok(format_number(evaluate(expression)?))
}

pub fn format_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let mut s = format!("{value:.12}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    depth: usize,
    ops: usize,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self
            .input
            .get(self.pos)
            .map(|b| b.is_ascii_whitespace())
            .unwrap_or(false)
        {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn consume(&mut self, byte: u8) -> bool {
        self.skip_ws();
        if self.peek() == Some(byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, byte: u8) -> Result<()> {
        if self.consume(byte) {
            Ok(())
        } else {
            bail!("Expected '{}' at byte {}", byte as char, self.pos)
        }
    }

    fn count_op(&mut self) -> Result<()> {
        self.ops += 1;
        if self.ops > MAX_OPS {
            bail!("Expression is too complex");
        }
        Ok(())
    }

    fn enter(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            bail!("Expression nesting is too deep");
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn parse_expr(&mut self) -> Result<f64> {
        self.enter()?;
        let mut value = self.parse_term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'+') => {
                    self.pos += 1;
                    self.count_op()?;
                    value += self.parse_term()?;
                }
                Some(b'-') => {
                    self.pos += 1;
                    self.count_op()?;
                    value -= self.parse_term()?;
                }
                _ => break,
            }
        }
        self.leave();
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<f64> {
        let mut value = self.parse_unary()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    self.count_op()?;
                    value *= self.parse_unary()?;
                }
                Some(b'/') => {
                    self.pos += 1;
                    self.count_op()?;
                    let rhs = self.parse_unary()?;
                    if rhs == 0.0 {
                        bail!("Division by zero");
                    }
                    value /= rhs;
                }
                Some(b'%') => {
                    self.pos += 1;
                    self.count_op()?;
                    let rhs = self.parse_unary()?;
                    if rhs == 0.0 {
                        bail!("Modulo by zero");
                    }
                    value %= rhs;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_unary(&mut self) -> Result<f64> {
        self.skip_ws();
        match self.peek() {
            Some(b'+') => {
                self.pos += 1;
                self.parse_unary()
            }
            Some(b'-') => {
                self.pos += 1;
                Ok(-self.parse_unary()?)
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<f64> {
        let base = self.parse_primary()?;
        self.skip_ws();
        if self.consume(b'^') {
            self.count_op()?;
            // Right-associative: 2^3^2 == 2^(3^2).
            let exponent = self.parse_unary()?;
            Ok(base.powf(exponent))
        } else {
            Ok(base)
        }
    }

    fn parse_primary(&mut self) -> Result<f64> {
        self.skip_ws();
        match self.peek() {
            Some(b'(') => {
                self.pos += 1;
                let value = self.parse_expr()?;
                self.expect(b')')?;
                Ok(value)
            }
            Some(b) if b.is_ascii_digit() || b == b'.' => self.parse_number(),
            Some(b) if b.is_ascii_alphabetic() || b == b'_' => self.parse_ident(),
            Some(b) => bail!("Unexpected character '{}' at byte {}", b as char, self.pos),
            None => bail!("Unexpected end of expression"),
        }
    }

    fn parse_number(&mut self) -> Result<f64> {
        self.skip_ws();
        let start = self.pos;
        let mut saw_digit = false;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                saw_digit = true;
                self.pos += 1;
            } else if b == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if !saw_digit {
            bail!("Invalid number at byte {start}");
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            let exp_start = self.pos;
            while self.peek().map(|b| b.is_ascii_digit()).unwrap_or(false) {
                self.pos += 1;
            }
            if exp_start == self.pos {
                bail!("Invalid exponent at byte {start}");
            }
        }
        let text = std::str::from_utf8(&self.input[start..self.pos])?;
        let value: f64 = text
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid number '{text}'"))?;
        Ok(value)
    }

    fn parse_ident(&mut self) -> Result<f64> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let name = std::str::from_utf8(&self.input[start..self.pos])?.to_ascii_lowercase();
        self.skip_ws();
        if self.consume(b'(') {
            let mut args = Vec::new();
            if !self.consume(b')') {
                loop {
                    args.push(self.parse_expr()?);
                    if self.consume(b')') {
                        break;
                    }
                    self.expect(b',')?;
                }
            }
            return call_function(&name, &args);
        }
        match name.as_str() {
            "pi" => Ok(std::f64::consts::PI),
            "e" => Ok(std::f64::consts::E),
            _ => bail!("Unknown identifier '{name}'"),
        }
    }
}

fn call_function(name: &str, args: &[f64]) -> Result<f64> {
    let one = |f: fn(f64) -> f64| -> Result<f64> {
        if args.len() != 1 {
            bail!("{name}() expects exactly 1 argument");
        }
        Ok(f(args[0]))
    };
    match name {
        "sqrt" => one(f64::sqrt),
        "sin" => one(f64::sin),
        "cos" => one(f64::cos),
        "tan" => one(f64::tan),
        "asin" => one(f64::asin),
        "acos" => one(f64::acos),
        "atan" => one(f64::atan),
        "ln" | "log" => one(f64::ln),
        "log10" => one(f64::log10),
        "log2" => one(f64::log2),
        "exp" => one(f64::exp),
        "abs" => one(f64::abs),
        "floor" => one(f64::floor),
        "ceil" => one(f64::ceil),
        "round" => one(f64::round),
        "pow" => {
            if args.len() != 2 {
                bail!("pow() expects exactly 2 arguments");
            }
            Ok(args[0].powf(args[1]))
        }
        "min" | "max" => {
            if args.is_empty() {
                bail!("{name}() expects at least 1 argument");
            }
            let mut acc = args[0];
            for v in &args[1..] {
                acc = if name == "min" {
                    acc.min(*v)
                } else {
                    acc.max(*v)
                };
            }
            Ok(acc)
        }
        _ => bail!("Unknown function '{name}'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_precedence() {
        assert_eq!(evaluate_to_string("2 + 3 * 4").unwrap(), "14");
        assert_eq!(evaluate_to_string("(2 + 3) * 4").unwrap(), "20");
        assert_eq!(evaluate_to_string("2^3^2").unwrap(), "512");
        assert_eq!(evaluate_to_string("-2^2").unwrap(), "-4");
    }

    #[test]
    fn functions_and_constants() {
        assert_eq!(evaluate_to_string("sqrt(144)").unwrap(), "12");
        assert_eq!(evaluate_to_string("min(3, 5, 1)").unwrap(), "1");
        assert_eq!(evaluate_to_string("floor(3.9)").unwrap(), "3");
        assert!((evaluate("sin(pi/2)").unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_code_injection() {
        assert!(evaluate("__import__('os').system('id')").is_err());
        assert!(evaluate("1); import os; os.system('id'").is_err());
        assert!(evaluate("print(1)").is_err());
        assert!(evaluate("1/0").is_err());
        assert!(evaluate(&"1+".repeat(400)).is_err());
    }
}
