//! The calculator's arithmetic: `+`, `−`, `×` and `÷` on decimal numbers, with × and ÷ first. The
//! expression is kept as it's keyed in, with `*` and `/` for the two that take a symbol; nothing
//! here reads the controller or the game.

/// Largest answer, in absolute value. Past it, a sum says it's too large.
const LIMIT: f64 = 1e12;
/// Decimal places an answer keeps.
const PLACES: usize = 10;

const INCOMPLETE: &str = "Finish the sum";
const DIVIDE_BY_ZERO: &str = "Can't divide by zero";
const TOO_LARGE: &str = "Too large";
const NOTHING: &str = "Nothing to send yet";

/// A key on the calculator's keypad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Digit(char),
    /// `+`, `-`, `*` or `/`.
    Op(char),
    Dot,
    Backspace,
    Clear,
    Equals,
}

/// What the calculator is showing: the expression keyed in so far, and why the last `=` failed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Calculator {
    expr: String,
    error: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Num(f64),
    Op(char),
}

fn is_op(c: char) -> bool {
    matches!(c, '+' | '-' | '*' | '/')
}

impl Calculator {
    pub fn press(&mut self, key: Press) {
        self.error = None;
        match key {
            Press::Digit(c) => self.digit(c),
            Press::Dot => self.dot(),
            Press::Op(op) => self.op(op),
            Press::Backspace => drop(self.expr.pop()),
            Press::Clear => self.expr.clear(),
            Press::Equals => match evaluate(&self.expr) {
                Ok(value) => self.expr = format_value(value),
                Err(message) => self.error = Some(message),
            },
        }
    }

    /// Shows `message` in place of the expression until the next key.
    pub fn fail(&mut self, message: &'static str) {
        self.error = Some(message);
    }

    /// What the answer is, for sending or copying: the expression as far as it goes, so a sum
    /// left with an operator gives the number before it.
    pub fn answer(&self) -> Result<String, &'static str> {
        let expr = self.expr.trim_end_matches(is_op);
        if expr.is_empty() {
            return Err(NOTHING);
        }
        evaluate(expr).map(format_value)
    }

    /// The line above the keypad: the error, or the expression with its answer after `=`.
    pub fn line(&self) -> String {
        if let Some(message) = self.error {
            return message.into();
        }
        if self.expr.is_empty() {
            return "0".into();
        }
        let shown = display(&self.expr);
        match self.answer() {
            Ok(answer) if answer != shown && answer != self.expr => format!("{shown} = {answer}"),
            _ => shown,
        }
    }

    fn digit(&mut self, c: char) {
        if self.current_number() == "0" {
            self.expr.pop();
        }
        self.expr.push(c);
    }

    fn dot(&mut self) {
        let number = self.current_number();
        if number.contains('.') {
            return;
        }
        if number.is_empty() {
            self.expr.push('0');
        }
        self.expr.push('.');
    }

    /// Pressing an operator after another replaces it. A minus on its own, at the start, is a
    /// sign rather than an operator, so nothing else follows it straight away.
    fn op(&mut self, op: char) {
        match self.expr.as_str() {
            "" if op == '-' => self.expr.push('-'),
            "" | "-" => {}
            expr if expr.ends_with(is_op) => {
                self.expr.pop();
                self.expr.push(op);
            }
            _ => self.expr.push(op),
        }
    }

    /// The digits of the number being keyed in, after its operator (or its sign, at the start).
    fn current_number(&self) -> &str {
        self.expr.rsplit(is_op).next().unwrap_or_default()
    }
}

/// The symbol an operator is shown with.
pub fn symbol(op: char) -> char {
    match op {
        '*' => '×',
        '/' => '÷',
        '-' => '−',
        c => c,
    }
}

/// The expression with the symbols it's shown with.
fn display(expr: &str) -> String {
    expr.chars().map(symbol).collect()
}

/// An answer as text: up to [`PLACES`] decimals, without trailing zeros.
fn format_value(value: f64) -> String {
    let text = format!("{value:.PLACES$}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" { "0".into() } else { text.into() }
}

/// Works out the value of `expr`, with × and ÷ before + and −.
fn evaluate(expr: &str) -> Result<f64, &'static str> {
    let tokens = tokenize(expr)?;
    let (mut total, mut sign, mut i) = (0.0, 1.0, 0);
    loop {
        let Some(Token::Num(first)) = tokens.get(i) else { return Err(INCOMPLETE) };
        let mut term = *first;
        i += 1;
        while let Some(Token::Op(op @ ('*' | '/'))) = tokens.get(i) {
            let Some(Token::Num(n)) = tokens.get(i + 1) else { return Err(INCOMPLETE) };
            if *op == '/' {
                if *n == 0.0 {
                    return Err(DIVIDE_BY_ZERO);
                }
                term /= n;
            } else {
                term *= n;
            }
            i += 2;
        }
        total += sign * term;
        match tokens.get(i) {
            None => break,
            Some(Token::Op(op @ ('+' | '-'))) => {
                sign = if *op == '+' { 1.0 } else { -1.0 };
                i += 1;
            }
            Some(_) => return Err(INCOMPLETE),
        }
    }
    if total.is_finite() && total.abs() <= LIMIT { Ok(total) } else { Err(TOO_LARGE) }
}

/// Splits `expr` into numbers and operators. A minus that starts a number is its sign.
fn tokenize(expr: &str) -> Result<Vec<Token>, &'static str> {
    let mut tokens = Vec::new();
    let mut number = String::new();
    for c in expr.chars() {
        if c.is_ascii_digit() || c == '.' || (c == '-' && number.is_empty()) {
            number.push(c);
        } else if is_op(c) {
            if !number.is_empty() {
                tokens.push(Token::Num(parse_number(&number)?));
                number.clear();
            }
            tokens.push(Token::Op(c));
        }
    }
    if !number.is_empty() {
        tokens.push(Token::Num(parse_number(&number)?));
    }
    Ok(tokens)
}

fn parse_number(text: &str) -> Result<f64, &'static str> {
    let digits = text.trim_start_matches('-');
    if digits.is_empty() || digits == "." {
        return Err(INCOMPLETE);
    }
    text.parse().map_err(|_| INCOMPLETE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keys in a row, as a keypad would press them: digits, dots and operators are one key each.
    fn keyed(keys: &str) -> Calculator {
        let mut calc = Calculator::default();
        for c in keys.chars() {
            calc.press(match c {
                '=' => Press::Equals,
                '<' => Press::Backspace,
                'C' => Press::Clear,
                '.' => Press::Dot,
                '+' | '-' | '*' | '/' => Press::Op(c),
                d => Press::Digit(d),
            });
        }
        calc
    }

    #[test]
    fn multiplication_and_division_come_before_addition_and_subtraction() {
        assert_eq!(evaluate("2+3*4"), Ok(14.0));
        assert_eq!(evaluate("10-6/3"), Ok(8.0));
        assert_eq!(evaluate("2*3-4/2*2"), Ok(2.0));
    }

    #[test]
    fn a_leading_minus_is_a_sign() {
        assert_eq!(evaluate("-5+2"), Ok(-3.0));
        assert_eq!(evaluate("-2*-3"), Ok(6.0));
        assert_eq!(keyed("-5").answer(), Ok("-5".into()));
    }

    #[test]
    fn decimals_are_shown_without_trailing_zeros() {
        assert_eq!(format_value(2.5), "2.5");
        assert_eq!(format_value(7.0 / 3.0), "2.3333333333");
        assert_eq!(format_value(-0.0), "0");
        assert_eq!(format_value(100.0), "100");
        assert_eq!(keyed("1.5*2").answer(), Ok("3".into()));
    }

    #[test]
    fn dividing_by_zero_is_an_error_not_an_answer() {
        assert_eq!(evaluate("1/0"), Err(DIVIDE_BY_ZERO));
        assert_eq!(keyed("1/0=").line(), DIVIDE_BY_ZERO);
    }

    #[test]
    fn an_answer_too_large_is_refused() {
        assert_eq!(evaluate("9999999999999*9999"), Err(TOO_LARGE));
    }

    #[test]
    fn an_unfinished_sum_gives_the_number_before_its_operator() {
        assert_eq!(keyed("12+").answer(), Ok("12".into()));
        assert_eq!(keyed("").answer(), Err(NOTHING));
        assert_eq!(keyed("1+").answer(), Ok("1".into()));
        assert_eq!(evaluate("1+"), Err(INCOMPLETE));
    }

    #[test]
    fn equals_replaces_the_sum_with_its_answer_so_it_goes_on() {
        let mut calc = keyed("2+3=");
        assert_eq!(calc.line(), "5");
        calc.press(Press::Op('*'));
        calc.press(Press::Digit('4'));
        assert_eq!(calc.line(), "5×4 = 20");
    }

    #[test]
    fn the_line_shows_the_sum_and_its_answer() {
        assert_eq!(keyed("7*8").line(), "7×8 = 56");
        assert_eq!(keyed("").line(), "0");
        assert_eq!(keyed("42").line(), "42");
    }

    #[test]
    fn a_second_dot_in_a_number_is_ignored_and_a_leading_dot_gets_a_zero() {
        assert_eq!(keyed("1.2.3").answer(), Ok("1.23".into()));
        assert_eq!(keyed(".5").answer(), Ok("0.5".into()));
    }

    #[test]
    fn an_operator_replaces_the_one_before_it() {
        assert_eq!(keyed("2+*3").answer(), Ok("6".into()));
        assert_eq!(keyed("-+").line(), "−");
    }

    #[test]
    fn a_digit_replaces_a_leading_zero() {
        assert_eq!(keyed("05").answer(), Ok("5".into()));
    }

    #[test]
    fn backspace_and_clear_undo_keys() {
        assert_eq!(keyed("12<").answer(), Ok("1".into()));
        assert_eq!(keyed("12C").line(), "0");
    }

    #[test]
    fn a_key_clears_an_error() {
        let mut calc = keyed("1/0=");
        calc.press(Press::Clear);
        assert_eq!(calc.line(), "0");
    }
}
