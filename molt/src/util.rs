//! Internal Utilities
//!
//! This module contains function for use by molt only.

use crate::tokenizer::Tokenizer;
use crate::types::*;
use std::cmp::Ordering;

/// Matches a string using Tcl's glob-style pattern syntax.
///
/// The matcher walks UTF-8 byte indices directly and retains only the most recent `*`
/// backtracking point.  Consequently the common path does not allocate and each pattern atom
/// is decoded only when it is visited.
pub(crate) fn glob_match(pattern: &str, text: &str, nocase: bool) -> bool {
    let mut pattern_index = 0;
    let mut text_index = 0;
    let mut star = None;

    loop {
        while char_at(pattern, pattern_index).is_some_and(|(ch, _)| ch == '*') {
            pattern_index = char_at(pattern, pattern_index).expect("star is present").1;
            star = Some((pattern_index, text_index));
        }

        if pattern_index == pattern.len() {
            if text_index == text.len() {
                return true;
            }
        } else if let Some((text_char, next_text)) = char_at(text, text_index) {
            let (matches, next_pattern) =
                match_atom(pattern, pattern_index, text_char, nocase);
            if matches {
                pattern_index = next_pattern;
                text_index = next_text;
                continue;
            }
        }

        let Some((after_star, previous_text)) = star else {
            return false;
        };
        let Some((_, next_text)) = char_at(text, previous_text) else {
            return false;
        };
        text_index = next_text;
        pattern_index = after_star;
        star = Some((after_star, next_text));
    }
}

fn char_at(value: &str, index: usize) -> Option<(char, usize)> {
    value[index..].chars().next().map(|ch| (ch, index + ch.len_utf8()))
}

fn match_atom(pattern: &str, index: usize, text: char, nocase: bool) -> (bool, usize) {
    let (atom, mut next) = char_at(pattern, index).expect("pattern index is in bounds");
    match atom {
        '?' => (true, next),
        '\\' => {
            if let Some((escaped, after)) = char_at(pattern, next) {
                (chars_equal(escaped, text, nocase), after)
            } else {
                (chars_equal('\\', text, nocase), next)
            }
        }
        '[' => {
            let mut matched = false;
            while let Some((first, after_first)) = class_char(pattern, next) {
                if first == ']' {
                    next = after_first;
                    break;
                }
                let range = char_at(pattern, after_first)
                    .filter(|(dash, _)| *dash == '-')
                    .and_then(|(_, after_dash)| class_char(pattern, after_dash))
                    .filter(|(last, _)| *last != ']');
                if let Some((last, after_last)) = range {
                    let text = case_key(text, nocase);
                    let first = case_key(first, nocase);
                    let last = case_key(last, nocase);
                    matched |= first <= text && text <= last;
                    next = after_last;
                } else {
                    matched |= chars_equal(first, text, nocase);
                    next = after_first;
                }
            }
            (matched, next)
        }
        _ => (chars_equal(atom, text, nocase), next),
    }
}

fn class_char(pattern: &str, index: usize) -> Option<(char, usize)> {
    let (ch, next) = char_at(pattern, index)?;
    if ch == '\\' {
        char_at(pattern, next).or(Some(('\\', next)))
    } else {
        Some((ch, next))
    }
}

fn chars_equal(left: char, right: char, nocase: bool) -> bool {
    left == right || (nocase && left.to_lowercase().eq(right.to_lowercase()))
}

fn case_key(ch: char, nocase: bool) -> char {
    if nocase {
        ch.to_lowercase().next().unwrap_or(ch)
    } else {
        ch
    }
}

pub fn is_varname_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Returns the character length of an unbraced Tcl variable name prefix.
pub fn varname_len(input: &str) -> usize {
    let mut index = 0;
    let mut characters = 0;
    while index < input.len() {
        let rest = &input[index..];
        if rest.starts_with("::") {
            index += 2;
            characters += 2;
        } else if let Some(ch) = rest.chars().next().filter(|ch| is_varname_char(*ch)) {
            index += ch.len_utf8();
            characters += 1;
        } else {
            break;
        }
    }
    characters
}

/// Reads the integer string from the head of the input.  If the function returns `Some`,
/// the value is the integer string that was read, and the `ptr` points to the following
/// character. Otherwise the `ptr` will be unchanged.
///
/// The string may consist of:
///
/// * A unary plus or minus
/// * One or more decimal digits.
///
/// ## Notes
///
/// * The resulting string has the form of an integer, but might be out of the valid range.
pub fn read_int(ptr: &mut Tokenizer) -> Option<String> {
    let mut p = ptr.clone();
    let mut result = String::new();
    let mut missing_digits = true;

    // FIRST, skip a unary operator.
    if p.is('+') || p.is('-') {
        result.push(p.next().unwrap());
    }

    // NEXT, skip a "0x".
    let mut radix = 10;

    if p.is('0') {
        result.push(p.next().unwrap());

        if p.is('x') {
            result.push(p.next().unwrap());
            radix = 16;
        } else {
            missing_digits = false;
        }
    }

    // NEXT, read the digits
    while p.has(|ch| ch.is_digit(radix)) {
        missing_digits = false;
        result.push(p.next().unwrap());
    }

    if result.is_empty() || missing_digits {
        None
    } else {
        ptr.skip_over(result.len());
        Some(result)
    }
}

/// Reads the floating point string from the head of the input.  If the function returns `Some`,
/// the value is the string that was read, and the `ptr` points to the following character.
/// Otherwise the `ptr` will be unchanged.
///
/// The string will consist of:
///
/// * Possibly, a unary plus/minus
/// * "Inf" (case insensitive), -OR-
/// * A number:
///   * Some number of decimal digits, optionally containing a ".".
///   * An optional exponent beginning with "e" or "E"
///   * The exponent may contain a + or -, followed by some number of digits.
///
/// ## Notes
///
/// * The resulting string has the form of a floating point number but might be out of the
///   valid range.
pub fn read_float(ptr: &mut Tokenizer) -> Option<String> {
    let mut p = ptr.clone();
    let mut result = String::new();
    let mut missing_mantissa = true;
    let mut missing_exponent = false;

    // FIRST, skip a unary operator.
    if p.is('+') || p.is('-') {
        result.push(p.next().unwrap());
    }

    // NEXT, looking for Inf
    if p.is('I') || p.is('i') {
        result.push(p.next().unwrap());

        if p.is('N') || p.is('n') {
            result.push(p.next().unwrap());
        } else {
            return None;
        }

        if p.is('F') || p.is('f') {
            result.push(p.next().unwrap());
            // Update the pointer.
            ptr.skip_over(result.len());
            return Some(result);
        } else {
            return None;
        }
    }

    // NEXT, get any integer digits
    while p.has(|ch| ch.is_ascii_digit()) {
        missing_mantissa = false;
        result.push(p.next().unwrap());
    }

    // NEXT, get any fractional part.
    if p.is('.') {
        result.push(p.next().unwrap());

        while p.has(|ch| ch.is_ascii_digit()) {
            missing_mantissa = false;
            result.push(p.next().unwrap());
        }
    }

    // NEXT, get any exponent.
    if p.is('e') || p.is('E') {
        missing_exponent = true;
        result.push(p.next().unwrap());

        if p.is('+') || p.is('-') {
            result.push(p.next().unwrap());
        }

        while p.has(|ch| ch.is_ascii_digit()) {
            missing_exponent = false;
            result.push(p.next().unwrap());
        }
    }

    if result.is_empty() || missing_mantissa || missing_exponent {
        None
    } else {
        // Update the pointer.
        ptr.skip_over(result.len());
        Some(result)
    }
}

/// Compare two strings, up to an optional length, returning -1, 0, or 1 as a
/// molt result.
pub(crate) fn compare_len(
    str1: &str,
    str2: &str,
    length: Option<MoltInt>,
) -> Result<MoltInt, Exception> {
    let s1;
    let s2;

    if let Some(len) = length {
        s1 = str1.substring(0, len as usize);
        s2 = str2.substring(0, len as usize);
    } else {
        s1 = str1;
        s2 = str2;
    }

    match s1.cmp(s2) {
        Ordering::Less => Ok(-1),
        Ordering::Equal => Ok(0),
        Ordering::Greater => Ok(1),
    }
}

pub(crate) trait StringUtils {
    fn substring(&self, start: usize, len: usize) -> &str;
}

impl StringUtils for str {
    fn substring(&self, start: usize, len: usize) -> &str {
        let mut char_pos = 0;
        let mut byte_start = 0;
        let mut it = self.chars();
        loop {
            if char_pos == start {
                break;
            }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_start += c.len_utf8();
            } else {
                break;
            }
        }
        char_pos = 0;
        let mut byte_end = byte_start;
        loop {
            if char_pos == len {
                break;
            }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_end += c.len_utf8();
            } else {
                break;
            }
        }
        &self[byte_start..byte_end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("a*b?d", "axxxbyd", false));
        assert!(glob_match("[a-c]at", "bat", false));
        assert!(glob_match("[abc", "b", false));
        assert!(glob_match(r"a\*b", "a*b", false));
        assert!(glob_match("К*", "космос", true));
        assert!(!glob_match("[a-c]at", "dat", false));
        assert!(!glob_match("a*b?d", "axxbd", false));
    }

    #[test]
    fn test_util_read_int() {
        let mut p = Tokenizer::new("abc");
        assert_eq!(None, read_int(&mut p));
        assert_eq!(Some('a'), p.peek());

        let mut p = Tokenizer::new("-abc");
        assert_eq!(None, read_int(&mut p));
        assert_eq!(Some('-'), p.peek());

        let mut p = Tokenizer::new("+abc");
        assert_eq!(None, read_int(&mut p));
        assert_eq!(Some('+'), p.peek());

        let mut p = Tokenizer::new("123");
        assert_eq!(Some("123".into()), read_int(&mut p));
        assert_eq!(None, p.peek());

        let mut p = Tokenizer::new("123abc");
        assert_eq!(Some("123".into()), read_int(&mut p));
        assert_eq!(Some('a'), p.peek());

        let mut p = Tokenizer::new("+123abc");
        assert_eq!(Some("+123".into()), read_int(&mut p));
        assert_eq!(Some('a'), p.peek());

        let mut p = Tokenizer::new("-123abc");
        assert_eq!(Some("-123".into()), read_int(&mut p));
        assert_eq!(Some('a'), p.peek());
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_util_read_float() {
        let mut p = Tokenizer::new("abc");
        assert_eq!(None, read_float(&mut p));
        assert_eq!(Some('a'), p.peek());

        let mut p = Tokenizer::new("-abc");
        assert_eq!(None, read_float(&mut p));
        assert_eq!(Some('-'), p.peek());

        let mut p = Tokenizer::new("+abc");
        assert_eq!(None, read_float(&mut p));
        assert_eq!(Some('+'), p.peek());

        let mut p = Tokenizer::new("123");
        assert_eq!(Some("123".into()), read_float(&mut p));
        assert_eq!(None, p.peek());

        let mut p = Tokenizer::new("123abc");
        assert_eq!(Some("123".into()), read_float(&mut p));
        assert_eq!(Some('a'), p.peek());

        let mut p = Tokenizer::new("123.");
        assert_eq!(Some("123.".into()), read_float(&mut p));
        assert_eq!(None, p.peek());

        let mut p = Tokenizer::new(".123");
        assert_eq!(Some(".123".into()), read_float(&mut p));
        assert_eq!(None, p.peek());

        let mut p = Tokenizer::new("123.123");
        assert_eq!(Some("123.123".into()), read_float(&mut p));
        assert_eq!(None, p.peek());

        let mut p = Tokenizer::new("1e5");
        assert_eq!(Some("1e5".into()), read_float(&mut p));
        assert_eq!(None, p.peek());

        let mut p = Tokenizer::new("1e+5");
        assert_eq!(Some("1e+5".into()), read_float(&mut p));
        assert_eq!(None, p.peek());

        let mut p = Tokenizer::new("1e-5");
        assert_eq!(Some("1e-5".into()), read_float(&mut p));
        assert_eq!(None, p.peek());

        let mut p = Tokenizer::new("1.1e1a");
        assert_eq!(Some("1.1e1".into()), read_float(&mut p));
        assert_eq!(Some('a'), p.peek());

        let mut p = Tokenizer::new("+123abc");
        assert_eq!(Some("+123".into()), read_float(&mut p));
        assert_eq!(Some('a'), p.peek());

        let mut p = Tokenizer::new("-123abc");
        assert_eq!(Some("-123".into()), read_float(&mut p));
        assert_eq!(Some('a'), p.peek());
    }
}
