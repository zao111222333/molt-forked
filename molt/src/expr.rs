//! The Expr command and parser
//!
//! * Ultimately, the command should probably move to commands.rs.
//!   But this is convenient for now.

use crate::eval_ptr::EvalPtr;
use crate::interp::Interp;
use crate::list;
use crate::parser::Word;
use crate::tokenizer::Tokenizer;
use crate::*;

//------------------------------------------------------------------------------------------------
// Datum Representation

type DatumResult = Result<Datum, Exception>;

/// The value type.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum Type {
    Int,
    Float,
    String,
}

/// A parsed expression value. Each variant contains exactly the state valid for its type.
#[derive(Debug, PartialEq)]
pub(crate) enum Datum {
    Int(MoltInt),
    Float(MoltFloat),
    String(String),
}

impl Datum {
    fn none() -> Self {
        Self::String(String::new())
    }

    pub(crate) fn int(int: MoltInt) -> Self {
        Self::Int(int)
    }

    pub(crate) fn float(flt: MoltFloat) -> Self {
        Self::Float(flt)
    }

    fn string(string: &str) -> Self {
        Self::String(string.to_owned())
    }

    fn value_type(&self) -> Type {
        match self {
            Self::Int(_) => Type::Int,
            Self::Float(_) => Type::Float,
            Self::String(_) => Type::String,
        }
    }

    fn is_true(&self) -> bool {
        match self {
            Self::Int(value) => *value != 0,
            _ => panic!("Datum::is_true called for non-integer"),
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(self, Self::Int(_) | Self::Float(_))
    }

    fn into_string(self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::String(value) => value,
        }
    }
}

//------------------------------------------------------------------------------------------------
// Functions

#[derive(Debug, Clone, Copy)]
enum BuiltinFunc {
    Abs,
    Double,
    Int,
    Round,
}

/// Lexical token and operator kind. Keeping this as an enum prevents invalid numeric tags and
/// lets the compiler exhaustively check every dispatch site.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Token {
    Unknown,
    Value,
    OpenParen,
    CloseParen,
    Comma,
    End,
    Multiply,
    Divide,
    Modulo,
    Plus,
    Minus,
    LeftShift,
    RightShift,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
    Equal,
    NotEqual,
    StringEqual,
    StringNotEqual,
    In,
    NotIn,
    BitAnd,
    BitXor,
    BitOr,
    And,
    Or,
    Question,
    Colon,
    UnaryMinus,
    UnaryPlus,
    Not,
    BitNot,
}

impl Token {
    const fn precedence(self) -> i32 {
        match self {
            Self::Multiply | Self::Divide | Self::Modulo => 14,
            Self::Plus
            | Self::Minus
            | Self::UnaryMinus
            | Self::UnaryPlus
            | Self::Not
            | Self::BitNot => 13,
            Self::LeftShift | Self::RightShift => 12,
            Self::Less | Self::Greater | Self::LessOrEqual | Self::GreaterOrEqual => 11,
            Self::Equal | Self::NotEqual => 10,
            Self::StringEqual | Self::StringNotEqual => 9,
            Self::In | Self::NotIn => 8,
            Self::BitAnd => 7,
            Self::BitXor => 6,
            Self::BitOr => 5,
            Self::And => 4,
            Self::Or => 3,
            Self::Question => 2,
            Self::Colon => 1,
            _ => 0,
        }
    }

    const fn is_binary(self) -> bool {
        matches!(
            self,
            Self::Multiply
                | Self::Divide
                | Self::Modulo
                | Self::Plus
                | Self::Minus
                | Self::LeftShift
                | Self::RightShift
                | Self::Less
                | Self::Greater
                | Self::LessOrEqual
                | Self::GreaterOrEqual
                | Self::Equal
                | Self::NotEqual
                | Self::StringEqual
                | Self::StringNotEqual
                | Self::In
                | Self::NotIn
                | Self::BitAnd
                | Self::BitXor
                | Self::BitOr
                | Self::And
                | Self::Or
                | Self::Question
                | Self::Colon
        )
    }

    const fn is_unary(self) -> bool {
        matches!(self, Self::UnaryMinus | Self::UnaryPlus | Self::Not | Self::BitNot)
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown token",
            Self::Value => "value",
            Self::OpenParen => "(",
            Self::CloseParen => ")",
            Self::Comma => ",",
            Self::End => "end of expression",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulo => "%",
            Self::Plus | Self::UnaryPlus => "+",
            Self::Minus | Self::UnaryMinus => "-",
            Self::LeftShift => "<<",
            Self::RightShift => ">>",
            Self::Less => "<",
            Self::Greater => ">",
            Self::LessOrEqual => "<=",
            Self::GreaterOrEqual => ">=",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::StringEqual => "eq",
            Self::StringNotEqual => "ne",
            Self::In => "in",
            Self::NotIn => "ni",
            Self::BitAnd => "&",
            Self::BitXor => "^",
            Self::BitOr => "|",
            Self::And => "&&",
            Self::Or => "||",
            Self::Question => "?",
            Self::Colon => ":",
            Self::Not => "!",
            Self::BitNot => "~",
        }
    }
}

//------------------------------------------------------------------------------------------------
// Parsing Context

/// Context for expr parsing
struct ExprInfo<'a> {
    // The full expr.
    original_expr: &'a str,

    // The input iterator, e.g., the pointer to the next character.
    expr: Tokenizer<'a>,

    // Last token's type.
    token: Token,

    // No Evaluation if > 0
    no_eval: usize,
}

impl<'a> ExprInfo<'a> {
    fn new(expr: &'a str) -> Self {
        Self {
            original_expr: expr,
            expr: Tokenizer::new(expr),
            token: Token::Unknown,
            no_eval: 0,
        }
    }
}

//------------------------------------------------------------------------------------------------
// Public API

/// Evaluates an expression and returns its value.
pub fn expr<Ctx: 'static>(interp: &mut Interp<Ctx>, expr: &Value) -> MoltResult {
    let value = expr_top_level(interp, expr.as_str())?;

    match value {
        Datum::Int(value) => molt_ok!(Value::from(value)),
        Datum::Float(value) => molt_ok!(Value::from(value)),
        Datum::String(value) => molt_ok!(Value::from(value)),
    }
}

//------------------------------------------------------------------------------------------------
// Expression Internals

/// Provides top-level functionality shared by molt_expr_string, molt_expr_int, etc.
fn expr_top_level<Ctx: 'static>(interp: &mut Interp<Ctx>, string: &str) -> DatumResult {
    let info = &mut ExprInfo::new(string);

    let result = expr_get_value(interp, info, -1);

    match result {
        Ok(value) => {
            if info.token != Token::End {
                return molt_err!("syntax error in expression \"{}\"", string);
            }

            if matches!(value, Datum::Float(_)) {
                // TODO: check for NaN, INF, and throw IEEE floating point error.
            }

            Ok(value)
        }
        Err(exception) => match exception.code() {
            ResultCode::Break => molt_err!("invoked \"break\" outside of a loop"),
            ResultCode::Continue => molt_err!("invoked \"continue\" outside of a loop"),
            _ => Err(exception),
        },
    }
}

/// Parse a "value" from the remainder of the expression in info.
/// The `prec` is a precedence value; treat any unparenthesized operator
/// with precedence less than or equal to `prec` as the end of the
/// expression.
#[allow(clippy::collapsible_if)]
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::float_cmp)]
fn expr_get_value<Ctx: 'static>(
    interp: &mut Interp<Ctx>,
    info: &mut ExprInfo,
    prec: i32,
) -> DatumResult {
    // There are two phases to this procedure.  First, pick off an initial value.
    // Then, parse (binary operator, value) pairs until done.
    let mut got_op = false;
    let mut value = expr_lex(interp, info)?;
    let mut value2: Datum;
    let mut operator: Token;

    if info.token == Token::OpenParen {
        // Parenthesized sub-expression.
        value = expr_get_value(interp, info, -1)?;

        if info.token != Token::CloseParen {
            return molt_err!(
                "unmatched parentheses in expression \"{}\"",
                info.original_expr
            );
        }
    } else {
        if info.token == Token::Minus {
            info.token = Token::UnaryMinus;
        }

        if info.token == Token::Plus {
            info.token = Token::UnaryPlus;
        }

        if info.token.is_unary() {
            // Process unary operators
            operator = info.token;
            value = expr_get_value(interp, info, info.token.precedence())?;

            if info.no_eval == 0 {
                value = eval_unary(operator, value)?;
            }
            got_op = true;
        } else if info.token != Token::Value {
            return syntax_error(info);
        }
    }

    // Got the first operand.  Now fetch (operator, operand) pairs

    if !got_op {
        // This reads the next token, which we expect to be an operator.
        // All we really care about is the token enum; if it's a value, it doesn't matter
        // what the value is.
        let _ = expr_lex(interp, info)?;
    }

    loop {
        operator = info.token;
        // ??? value2.pv.next = value2.pv.buffer;

        if !operator.is_binary() {
            if operator == Token::End
                || operator == Token::CloseParen
                || operator == Token::Comma
            {
                return Ok(value);
            } else {
                return syntax_error(info);
            }
        }

        if operator.precedence() <= prec {
            return Ok(value);
        }

        // If we're doing an Token::And or Token::Or and the first operand already determines
        // the result, don't execute anything in the second operand: just parse.
        // Same style for ?: pairs.

        if operator == Token::And || operator == Token::Or || operator == Token::Question
        {
            // For these operators, we need an integer value.  Convert or return
            // an error.
            value = match value {
                Datum::Int(value) => Datum::int(value),
                Datum::Float(value) => Datum::int(MoltInt::from(value != 0.0)),
                Datum::String(_) => {
                    if info.no_eval == 0 {
                        return illegal_type(Type::String, operator);
                    }
                    Datum::int(0)
                }
            };

            if (operator == Token::And && !value.is_true())
                || (operator == Token::Or && value.is_true())
            {
                // Short-circuit; we don't care about the next operand, but it must be
                // syntactically correct.
                info.no_eval += 1;
                let _ = expr_get_value(interp, info, operator.precedence())?;
                info.no_eval -= 1;

                if operator == Token::Or {
                    value = Datum::int(1);
                }

                // Go on to the next operator.
                continue;
            } else if operator == Token::Question {
                // Special note: ?: operators must associate right to left.  To make
                // this happen, use a precedence one lower than Token::Question when calling
                // expr_get_value recursively.
                if value.is_true() {
                    value =
                        expr_get_value(interp, info, Token::Question.precedence() - 1)?;

                    if info.token != Token::Colon {
                        return syntax_error(info);
                    }

                    info.no_eval += 1;
                    value2 =
                        expr_get_value(interp, info, Token::Question.precedence() - 1)?;
                    info.no_eval -= 1;
                } else {
                    info.no_eval += 1;
                    value2 =
                        expr_get_value(interp, info, Token::Question.precedence() - 1)?;
                    info.no_eval -= 1;

                    if info.token != Token::Colon {
                        return syntax_error(info);
                    }

                    value =
                        expr_get_value(interp, info, Token::Question.precedence() - 1)?;
                }
            } else {
                value2 = expr_get_value(interp, info, operator.precedence())?;
            }
        } else {
            value2 = expr_get_value(interp, info, operator.precedence())?;
        }

        if !info.token.is_binary()
            && !matches!(
                info.token,
                Token::Value | Token::End | Token::Comma | Token::CloseParen
            )
        {
            return syntax_error(info);
        }

        if info.no_eval > 0 {
            continue;
        }

        // Carry out the function of the specified operator.
        match operator {
            Token::Multiply
            | Token::Divide
            | Token::Modulo
            | Token::Plus
            | Token::Minus
            | Token::LeftShift
            | Token::RightShift => {
                value = eval_arithmetic(operator, value, value2)?;
            }
            Token::Less
            | Token::Greater
            | Token::LessOrEqual
            | Token::GreaterOrEqual
            | Token::Equal
            | Token::NotEqual => {
                value = Datum::int(MoltInt::from(compare(operator, value, value2)));
            }
            Token::StringEqual | Token::StringNotEqual => {
                let equal = value.into_string() == value2.into_string();
                value =
                    Datum::int(MoltInt::from(equal == (operator == Token::StringEqual)));
            }
            Token::In | Token::NotIn => {
                let needle = value.into_string();
                let list = list::get_list(&value2.into_string())?;
                let contains = list.iter().any(|item| item.as_str() == needle);
                value = Datum::int(MoltInt::from(contains == (operator == Token::In)));
            }
            Token::BitAnd | Token::BitXor | Token::BitOr => {
                value = eval_bitwise(operator, value, value2)?;
            }
            Token::And | Token::Or => {
                let left = numeric_truth(&value, operator)?;
                let right = numeric_truth(&value2, operator)?;
                let result =
                    if operator == Token::And { left && right } else { left || right };
                value = Datum::int(MoltInt::from(result));
            }

            Token::Colon => {
                return molt_err!("can't have : operator without ? first");
            }

            Token::Question => {}
            _ => unreachable!("non-binary token reached operator dispatch"),
        }
    }
}

fn eval_unary(operator: Token, value: Datum) -> DatumResult {
    match (operator, value) {
        (Token::UnaryMinus, Datum::Int(value)) => value
            .checked_neg()
            .map(Datum::int)
            .ok_or_else(|| Exception::molt_err(Value::from("integer overflow"))),
        (Token::UnaryMinus, Datum::Float(value)) => Ok(Datum::float(-value)),
        (Token::UnaryMinus, Datum::String(_)) => illegal_type(Type::String, operator),
        (Token::UnaryPlus, value) if value.is_numeric() => Ok(value),
        (Token::UnaryPlus, value) => illegal_type(value.value_type(), operator),
        (Token::Not, Datum::Int(value)) => Ok(Datum::int(MoltInt::from(value == 0))),
        (Token::Not, Datum::Float(value)) => Ok(Datum::int(MoltInt::from(value == 0.0))),
        (Token::Not, Datum::String(_)) => illegal_type(Type::String, operator),
        (Token::BitNot, Datum::Int(value)) => Ok(Datum::int(!value)),
        (Token::BitNot, value) => illegal_type(value.value_type(), operator),
        _ => unreachable!("non-unary token reached unary dispatch"),
    }
}

fn eval_arithmetic(operator: Token, left: Datum, right: Datum) -> DatumResult {
    match (left, right) {
        (Datum::String(_), _) | (_, Datum::String(_)) => {
            illegal_type(Type::String, operator)
        }
        (Datum::Int(left), Datum::Int(right)) => {
            let value = match operator {
                Token::Multiply => left.checked_mul(right),
                Token::Divide if right == 0 => return molt_err!("divide by zero"),
                Token::Divide => left.checked_div(right),
                Token::Modulo if right == 0 => return molt_err!("divide by zero"),
                Token::Modulo => left.checked_rem(right),
                Token::Plus => left.checked_add(right),
                Token::Minus => left.checked_sub(right),
                Token::LeftShift => Some(left << right),
                Token::RightShift => Some(left >> right),
                _ => return molt_err!("unknown operator in expression"),
            };
            value
                .map(Datum::int)
                .ok_or_else(|| Exception::molt_err(Value::from("integer overflow")))
        }
        (left, right) => {
            if matches!(operator, Token::Modulo | Token::LeftShift | Token::RightShift) {
                let bad_type = if matches!(left, Datum::Float(_)) {
                    Type::Float
                } else {
                    right.value_type()
                };
                return illegal_type(bad_type, operator);
            }

            let left = match left {
                Datum::Int(value) => value as MoltFloat,
                Datum::Float(value) => value,
                Datum::String(_) => unreachable!(),
            };
            let right = match right {
                Datum::Int(value) => value as MoltFloat,
                Datum::Float(value) => value,
                Datum::String(_) => unreachable!(),
            };
            if operator == Token::Divide && right == 0.0 {
                return molt_err!("divide by zero");
            }
            Ok(Datum::float(match operator {
                Token::Multiply => left * right,
                Token::Divide => left / right,
                Token::Plus => left + right,
                Token::Minus => left - right,
                _ => unreachable!(),
            }))
        }
    }
}

fn eval_bitwise(operator: Token, left: Datum, right: Datum) -> DatumResult {
    let left = match left {
        Datum::Int(value) => value,
        value => return illegal_type(value.value_type(), operator),
    };
    let right = match right {
        Datum::Int(value) => value,
        value => return illegal_type(value.value_type(), operator),
    };
    Ok(Datum::int(match operator {
        Token::BitAnd => left & right,
        Token::BitXor => left ^ right,
        Token::BitOr => left | right,
        _ => unreachable!(),
    }))
}

fn numeric_truth(value: &Datum, operator: Token) -> Result<bool, Exception> {
    match value {
        Datum::Int(value) => Ok(*value != 0),
        Datum::Float(value) => Ok(*value != 0.0),
        Datum::String(_) => illegal_type(Type::String, operator).map(|_| false),
    }
}

#[allow(clippy::float_cmp)]
fn compare(operator: Token, left: Datum, right: Datum) -> bool {
    match (left, right) {
        (Datum::String(left), right) => {
            compare_strings(operator, &left, &right.into_string())
        }
        (left, Datum::String(right)) => {
            compare_strings(operator, &left.into_string(), &right)
        }
        (Datum::Int(left), Datum::Int(right)) => compare_ints(operator, left, right),
        (left, right) => {
            let left = match left {
                Datum::Int(value) => value as MoltFloat,
                Datum::Float(value) => value,
                Datum::String(_) => unreachable!(),
            };
            let right = match right {
                Datum::Int(value) => value as MoltFloat,
                Datum::Float(value) => value,
                Datum::String(_) => unreachable!(),
            };
            match operator {
                Token::Less => left < right,
                Token::Greater => left > right,
                Token::LessOrEqual => left <= right,
                Token::GreaterOrEqual => left >= right,
                Token::Equal => left == right,
                Token::NotEqual => left != right,
                _ => unreachable!(),
            }
        }
    }
}

fn compare_ints(operator: Token, left: MoltInt, right: MoltInt) -> bool {
    match operator {
        Token::Less => left < right,
        Token::Greater => left > right,
        Token::LessOrEqual => left <= right,
        Token::GreaterOrEqual => left >= right,
        Token::Equal => left == right,
        Token::NotEqual => left != right,
        _ => unreachable!(),
    }
}

fn compare_strings(operator: Token, left: &str, right: &str) -> bool {
    match operator {
        Token::Less => left < right,
        Token::Greater => left > right,
        Token::LessOrEqual => left <= right,
        Token::GreaterOrEqual => left >= right,
        Token::Equal => left == right,
        Token::NotEqual => left != right,
        _ => unreachable!(),
    }
}

/// Lexical analyzer for the expression parser.  Parses a single value, operator, or other
/// syntactic element from an expression string.
///
/// ## Results
///
/// Returns an error result if an error occurs while doing lexical analysis or
/// executing an embedded command.  On success, info.token is set to the last token type,
/// and info is updated to point to the next token.  If the token is Token::Value, the returned
/// Datum contains it.
fn expr_lex<Ctx: 'static>(interp: &mut Interp<Ctx>, info: &mut ExprInfo) -> DatumResult {
    // FIRST, skip white space.
    let mut p = info.expr.clone();

    p.skip_while(|c| c.is_whitespace());

    if p.at_end() {
        info.token = Token::End;
        info.expr = p;
        return Ok(Datum::none());
    }

    // First try to parse the token as an integer or floating-point number.
    // Don't want to check for a number if the first character is "+"
    // or "-".  If we do, we might treat a binary operator as unary by
    // mistake, which will eventually cause a syntax error.

    if !p.is('+') && !p.is('-') {
        if expr_looks_like_int(&p) {
            // There's definitely an integer to parse; parse it.
            let token = util::read_int(&mut p).unwrap();
            let int = Value::get_int(&token)?;
            info.token = Token::Value;
            info.expr = p;
            return Ok(Datum::int(int));
        } else if let Some(token) = util::read_float(&mut p) {
            info.token = Token::Value;
            info.expr = p;
            return Ok(Datum::float(Value::get_float(&token)?));
        }
    }

    // It isn't a number, so the next character will determine what it is.
    info.expr = p.clone();
    info.expr.skip();

    match p.peek() {
        Some('$') => {
            let mut ctx = EvalPtr::from_tokenizer(&p);
            ctx.set_no_eval(info.no_eval > 0);
            let var_val = parse_and_eval_variable(interp, &mut ctx)?;
            info.token = Token::Value;
            info.expr = ctx.to_tokenizer();
            if info.no_eval > 0 {
                Ok(Datum::none())
            } else {
                expr_parse_value(&var_val)
            }
        }
        Some('[') => {
            let mut ctx = EvalPtr::from_tokenizer(&p);
            ctx.set_no_eval(info.no_eval > 0);
            let script_val = parse_and_eval_script(interp, &mut ctx)?;
            info.token = Token::Value;
            info.expr = ctx.to_tokenizer();
            if info.no_eval > 0 {
                Ok(Datum::none())
            } else {
                expr_parse_value(&script_val)
            }
        }
        Some('"') => {
            let mut ctx = EvalPtr::from_tokenizer(&p);
            ctx.set_no_eval(info.no_eval > 0);
            let val = parse_and_eval_quoted_word(interp, &mut ctx)?;
            info.token = Token::Value;
            info.expr = ctx.to_tokenizer();
            if info.no_eval > 0 {
                Ok(Datum::none())
            } else {
                // Note: we got a Value, but since it was parsed from a quoted string,
                // it won't already be numeric.
                expr_parse_string(val.as_str())
            }
        }
        Some('{') => {
            let mut ctx = EvalPtr::from_tokenizer(&p);
            ctx.set_no_eval(info.no_eval > 0);
            let val = parse_and_eval_braced_word(&mut ctx)?;
            info.token = Token::Value;
            info.expr = ctx.to_tokenizer();
            if info.no_eval > 0 {
                Ok(Datum::none())
            } else {
                // Note: we got a Value, but since it was parsed from a braced string,
                // it won't already be numeric.
                expr_parse_string(val.as_str())
            }
        }
        Some('(') => {
            info.token = Token::OpenParen;
            Ok(Datum::none())
        }
        Some(')') => {
            info.token = Token::CloseParen;
            Ok(Datum::none())
        }
        Some(',') => {
            info.token = Token::Comma;
            Ok(Datum::none())
        }
        Some('*') => {
            info.token = Token::Multiply;
            Ok(Datum::none())
        }
        Some('/') => {
            info.token = Token::Divide;
            Ok(Datum::none())
        }
        Some('%') => {
            info.token = Token::Modulo;
            Ok(Datum::none())
        }
        Some('+') => {
            info.token = Token::Plus;
            Ok(Datum::none())
        }
        Some('-') => {
            info.token = Token::Minus;
            Ok(Datum::none())
        }
        Some('?') => {
            info.token = Token::Question;
            Ok(Datum::none())
        }
        Some(':') => {
            info.token = Token::Colon;
            Ok(Datum::none())
        }
        Some('<') => {
            p.skip();
            match p.peek() {
                Some('<') => {
                    info.token = Token::LeftShift;
                    p.skip();
                    info.expr = p;
                    Ok(Datum::none())
                }
                Some('=') => {
                    info.token = Token::LessOrEqual;
                    p.skip();
                    info.expr = p;
                    Ok(Datum::none())
                }
                _ => {
                    info.token = Token::Less;
                    Ok(Datum::none())
                }
            }
        }
        Some('>') => {
            p.skip();
            match p.peek() {
                Some('>') => {
                    info.token = Token::RightShift;
                    p.skip();
                    info.expr = p;
                    Ok(Datum::none())
                }
                Some('=') => {
                    info.token = Token::GreaterOrEqual;
                    p.skip();
                    info.expr = p;
                    Ok(Datum::none())
                }
                _ => {
                    info.token = Token::Greater;
                    Ok(Datum::none())
                }
            }
        }
        Some('=') => {
            p.skip();
            if let Some('=') = p.peek() {
                info.token = Token::Equal;
                p.skip();
                info.expr = p;
            } else {
                info.token = Token::Unknown;
            }
            Ok(Datum::none())
        }
        Some('!') => {
            p.skip();
            if let Some('=') = p.peek() {
                info.token = Token::NotEqual;
                p.skip();
                info.expr = p;
            } else {
                info.token = Token::Not;
            }
            Ok(Datum::none())
        }
        Some('&') => {
            p.skip();
            if let Some('&') = p.peek() {
                info.token = Token::And;
                p.skip();
                info.expr = p;
            } else {
                info.token = Token::BitAnd;
            }
            Ok(Datum::none())
        }
        Some('^') => {
            info.token = Token::BitXor;
            Ok(Datum::none())
        }
        Some('|') => {
            p.skip();
            if let Some('|') = p.peek() {
                info.token = Token::Or;
                p.skip();
                info.expr = p;
            } else {
                info.token = Token::BitOr;
            }
            Ok(Datum::none())
        }
        Some('~') => {
            info.token = Token::BitNot;
            Ok(Datum::none())
        }
        Some(_) if p.has(|c| c.is_alphabetic()) => {
            let mut str = String::new();
            while p.has(|c| c.is_alphabetic() || c.is_ascii_digit()) {
                str.push(p.next().unwrap());
            }

            // NOTE: Could use get_boolean to test for the boolean constants, but it's
            // probably overkill.
            match str.as_ref() {
                "true" | "yes" | "on" => {
                    info.expr = p;
                    info.token = Token::Value;
                    Ok(Datum::int(1))
                }
                "false" | "no" | "off" => {
                    info.expr = p;
                    info.token = Token::Value;
                    Ok(Datum::int(0))
                }
                "eq" => {
                    info.expr = p;
                    info.token = Token::StringEqual;
                    Ok(Datum::none())
                }
                "ne" => {
                    info.expr = p;
                    info.token = Token::StringNotEqual;
                    Ok(Datum::none())
                }
                "in" => {
                    info.expr = p;
                    info.token = Token::In;
                    Ok(Datum::none())
                }
                "ni" => {
                    info.expr = p;
                    info.token = Token::NotIn;
                    Ok(Datum::none())
                }
                _ => {
                    info.expr = p;
                    expr_math_func(interp, info, &str)
                }
            }
        }
        Some(_) => {
            p.skip();
            info.expr = p;
            info.token = Token::Unknown;
            Ok(Datum::none())
        }
        None => {
            p.skip();
            info.expr = p;
            info.token = Token::Unknown;
            Ok(Datum::none())
        }
    }
}

// Parses a variable reference.  A bare "$" is an error.
fn parse_and_eval_variable<Ctx: 'static>(
    interp: &mut Interp<Ctx>,
    ctx: &mut EvalPtr,
) -> MoltResult {
    // FIRST, skip the '$'
    ctx.skip_char('$');

    // NEXT, make sure this is really a variable reference.
    if !ctx.next_is_varname_char() && !ctx.next_is('{') {
        return molt_err!("invalid character \"$\"");
    }

    // NEXT, get the variable reference.
    let word = parser::parse_varname(ctx)?;

    if ctx.is_no_eval() {
        Ok(Value::empty())
    } else {
        interp.eval_word(&word)
    }
}

/// Parses and evaluates an interpolated script in Molt input, i.e., a string beginning with
/// a "[", returning a MoltResult.  If the no_eval flag is set, returns an empty value.
/// This is used to handled interpolated scripts in expressions.
fn parse_and_eval_script<Ctx: 'static>(
    interp: &mut Interp<Ctx>,
    ctx: &mut EvalPtr,
) -> MoltResult {
    // FIRST, skip the '['
    ctx.skip_char('[');

    // NEXT, parse the script up to the matching ']'
    let old_flag = ctx.is_bracket_term();
    ctx.set_bracket_term(true);

    let script = parser::parse_script(ctx)?;
    let result =
        if ctx.is_no_eval() { Ok(Value::empty()) } else { interp.eval_script(&script) };

    ctx.set_bracket_term(old_flag);

    // NEXT, make sure there's a closing bracket
    if result.is_ok() {
        if ctx.next_is(']') {
            ctx.next();
        } else {
            return molt_err_uncompleted!("missing close-bracket");
        }
    }

    result
}

/// Parses and evaluates a quoted word in Molt input, i.e., a string beginning with
/// a double quote, returning a MoltResult.  If the no_eval flag is set, returns an empty
/// value.  This is used to handle double-quoted strings in expressions.
fn parse_and_eval_quoted_word<Ctx: 'static>(
    interp: &mut Interp<Ctx>,
    ctx: &mut EvalPtr,
) -> MoltResult {
    let word = parser::parse_quoted_word(ctx)?;

    if ctx.is_no_eval() {
        Ok(Value::empty())
    } else {
        interp.eval_word(&word)
    }
}

/// Parses a braced word, returning a Value.
fn parse_and_eval_braced_word(ctx: &mut EvalPtr) -> MoltResult {
    if let Word::Value(val) = parser::parse_braced_word(ctx)? {
        Ok(val)
    } else {
        unreachable!()
    }
}

/// Parses math functions, returning the evaluated value.
fn expr_math_func<Ctx>(
    interp: &mut Interp<Ctx>,
    info: &mut ExprInfo,
    func_name: &str,
) -> DatumResult {
    let function = BuiltinFunc::from_name(func_name)?;

    // NEXT, get the open paren.
    let _ = expr_lex(interp, info)?;

    if info.token != Token::OpenParen {
        return syntax_error(info);
    }

    let argument = expr_get_value(interp, info, -1)?;
    if info.token == Token::Comma {
        return molt_err!("too many arguments for math function");
    }
    if info.token != Token::CloseParen {
        return syntax_error(info);
    }
    if info.no_eval == 0 && !argument.is_numeric() {
        return molt_err!("argument to math function didn't have numeric value");
    }

    // NEXT, if we aren't evaluating, return an empty value.
    if info.no_eval > 0 {
        return Ok(Datum::none());
    }

    // NEXT, invoke the math function.
    info.token = Token::Value;
    function.execute(argument)
}

impl BuiltinFunc {
    fn from_name(name: &str) -> Result<Self, Exception> {
        match name {
            "abs" => Ok(Self::Abs),
            "double" => Ok(Self::Double),
            "int" => Ok(Self::Int),
            "round" => Ok(Self::Round),
            _ => molt_err!("unknown math function \"{}\"", name),
        }
    }

    fn execute(self, argument: Datum) -> DatumResult {
        match (self, argument) {
            (Self::Abs, Datum::Int(value)) => value
                .checked_abs()
                .map(Datum::int)
                .ok_or_else(|| Exception::molt_err(Value::from("integer overflow"))),
            (Self::Abs, Datum::Float(value)) => Ok(Datum::float(value.abs())),
            (Self::Double, Datum::Int(value)) => Ok(Datum::float(value as MoltFloat)),
            (Self::Double, Datum::Float(value)) => Ok(Datum::float(value)),
            (Self::Int, Datum::Int(value)) => Ok(Datum::int(value)),
            (Self::Int, Datum::Float(value)) => Ok(Datum::int(value as MoltInt)),
            (Self::Round, Datum::Int(value)) => Ok(Datum::int(value)),
            (Self::Round, Datum::Float(value)) if value < 0.0 => {
                Ok(Datum::int((value - 0.5) as MoltInt))
            }
            (Self::Round, Datum::Float(value)) => {
                Ok(Datum::int((value + 0.5) as MoltInt))
            }
            (_, Datum::String(_)) => {
                molt_err!("argument to math function didn't have numeric value")
            }
        }
    }
}

/// If the value already has a numeric data rep, just gets it as a Datum; otherwise,
/// tries to parse it out as a string.
///
/// NOTE: We don't just use `Value::as_float` or `Value::as_int`, as those expect
/// to parse strings with no extra whitespace.  (That may be a bug.)
fn expr_parse_value(value: &Value) -> DatumResult {
    match value.already_number() {
        Some(datum) => Ok(datum),
        _ => expr_parse_string(value.as_str()),
    }
}

/// Given a string (such as one coming from command or variable substitution) make a
/// Datum based on the string.  The value will be floating-point or integer if possible,
/// or else it will just be a copy of the string.  Returns an error on failed numeric
/// conversions.
fn expr_parse_string(string: &str) -> DatumResult {
    if !string.is_empty() {
        let mut p = Tokenizer::new(string);

        if expr_looks_like_int(&p) {
            // FIRST, skip leading whitespace.
            p.skip_while(|c| c.is_whitespace());

            // NEXT, get the integer token from it.  We know there has to be something,
            // since it "looks like int".
            let token = util::read_int(&mut p).unwrap();

            // NEXT, did we read the whole string?  If not, it isn't really an integer.
            // Otherwise, drop through and return it as a string.
            p.skip_while(|c| c.is_whitespace());

            if p.at_end() {
                // Can return an error if the number is too long to represent as a
                // MoltInt.  This is consistent with Tcl 7.6.  (Tcl 8 uses BigNums.)
                let int = Value::get_int(&token)?;
                return Ok(Datum::int(int));
            }
        } else {
            // FIRST, see if it's a double. Skip leading whitespace.
            p.skip_while(|c| c.is_whitespace());

            // NEXT, see if we can get a float token from it.
            if let Some(token) = util::read_float(&mut p) {
                // Did we read the whole string?  If not, it isn't really a float.
                // Otherwise, drop through and return it as a string.
                p.skip_while(|c| c.is_whitespace());

                if p.at_end() {
                    // Can theoretically return an error.  This is consistent with
                    // Tcl 7.6.  Molt and Tcl 8 return 0, Inf, or -Inf instead.
                    let flt = Value::get_float(&token)?;
                    return Ok(Datum::float(flt));
                }
            }
        }
    }

    Ok(Datum::string(string))
}

// Distinguished between decimal integers and floating-point values
fn expr_looks_like_int<'a>(ptr: &Tokenizer<'a>) -> bool {
    // FIRST, skip whitespace
    let mut p = ptr.clone();
    p.skip_while(|c| c.is_whitespace());

    if p.is('+') || p.is('-') {
        p.skip();
    }

    if !p.has(|ch| ch.is_ascii_digit()) {
        return false;
    }
    p.skip();

    while p.has(|ch| ch.is_ascii_digit()) {
        p.skip();
    }

    !p.is('.') && !p.is('e') && !p.is('E')
}

// Return standard syntax error
fn syntax_error(info: &mut ExprInfo) -> DatumResult {
    molt_err!("syntax error in expression \"{}\"", info.original_expr)
}

// Return standard illegal type error
fn illegal_type(bad_type: Type, op: Token) -> DatumResult {
    let type_str = if bad_type == Type::Float {
        "floating-point value"
    } else {
        "non-numeric string"
    };

    molt_err!("can't use {} as operand of \"{}\"", type_str, op.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_metadata_distinguishes_operator_kinds() {
        assert!(Token::Multiply.is_binary());
        assert!(!Token::Multiply.is_unary());
        assert!(Token::UnaryMinus.is_unary());
        assert!(!Token::UnaryMinus.is_binary());
        assert_eq!(Token::Question.precedence(), 2);
        assert_eq!(Token::Colon.precedence(), 1);
        assert_eq!(Token::StringEqual.as_str(), "eq");
        assert_eq!(Token::NotIn.as_str(), "ni");
    }

    #[test]
    fn integer_lookahead_preserves_expression_boundaries() {
        for value in ["1", "+1", "-1", "123", "123a"] {
            assert!(expr_looks_like_int(&Tokenizer::new(value)), "{value}");
        }
        for value in ["", "a", "123.", "123e", "123E", ".", "e", "E"] {
            assert!(!expr_looks_like_int(&Tokenizer::new(value)), "{value}");
        }
    }
}
