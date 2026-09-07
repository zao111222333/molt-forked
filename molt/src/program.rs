//! Reusable compiled Tcl programs.

use crate::{types::Exception, value::Value};

/// An opaque, reusable Tcl program.
///
/// `Program` keeps the original source string and the parser's internal form in
/// the same reference-counted [`Value`] cache used by normal Tcl evaluation.
/// Cloning a program is therefore inexpensive and does not reparse its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    value: Value,
}

impl Program {
    /// Compiles Tcl source without evaluating it.
    pub fn compile(source: &str) -> Result<Self, Exception> {
        let value = Value::from(source);
        value.as_script()?;
        Ok(Self { value })
    }

    /// Returns the exact source string from which this program was compiled.
    #[must_use]
    pub fn source(&self) -> &str {
        self.value.as_str()
    }

    pub(crate) const fn value(&self) -> &Value {
        &self.value
    }
}

/// Compiles Tcl source into a reusable opaque [`Program`].
pub fn compile(source: &str) -> Result<Program, Exception> {
    Program::compile(source)
}
