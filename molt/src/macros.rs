//! Convenience Macros
//!
//! This module contains macros for use by command authors.

/// Returns an `Ok` `MoltResult`.
///
/// If called with no arguments, returns an empty value as the `Ok` result.
/// If called with one argument, returns the argument as the `Ok` result, converting it
/// to a value automatically.
/// If called with two or more arguments, computes the `Ok` result using
/// `format!()`; the first argument is naturally the format string.
///
/// # Examples
///
/// ```
/// use molt_forked::*;
///
/// // Return the empty result
/// fn func1() -> MoltResult {
///     // ...
///     molt_ok!()
/// }
///
/// assert_eq!(func1(), Ok(Value::empty()));
///
/// // Return an arbitrary value
/// fn func2() -> MoltResult {
///     // ...
///     molt_ok!(17)
/// }
///
/// assert_eq!(func2(), Ok(17.into()));
///
/// // Return a formatted value
/// fn func3() -> MoltResult {
///     // ...
///     molt_ok!("The answer is {}", 17)
/// }
///
/// assert_eq!(func3(), Ok("The answer is 17".into()));
/// ```
#[macro_export]
macro_rules! molt_ok {
    () => (
        Ok($crate::Value::empty())
    );
    ($arg:expr) => (
        Ok($crate::Value::from($arg))
    );
    ($($arg:tt)*) => (
        Ok($crate::Value::from(format!($($arg)*)))
    )
}

/// Returns an `Exception`.  The error message is formatted
/// as with `format!()`.
///
/// If called with one argument, the single argument is used as the error message.
/// If called with more than one argument, the first is a `format!()` format string,
/// and the remainder are the values to format.
///
/// This macro wraps the [`Exception::molt_err`](types/struct.Exception.html#method.molt_err)
/// method.
///
/// # Examples
///
/// ```
/// use molt_forked::*;
/// # fn foo_fail() -> Result<Value, &'static str> { Err("...") }
///
/// // Return a simple error message
/// fn err1() -> MoltResult {
///     // ...
///     foo_fail().map_err(|e| molt_except!("error message: {}", e))
/// }
///
/// let result = err1();
/// assert!(result.is_err());
///
/// let exception = result.err().unwrap();
/// assert!(exception.is_error());
/// assert_eq!(exception.value(), "error message: ...".into());
/// ```
#[macro_export]
macro_rules! molt_except {
    ($arg:expr) => (
        $crate::Exception::molt_err($crate::Value::from($arg))
    );
    ($($arg:tt)*) => (
        $crate::Exception::molt_err($crate::Value::from(format!($($arg)*)))
    )
}

/// Returns an `Error` `MoltResult`.  The error message is formatted
/// as with `format!()`.
///
/// If called with one argument, the single argument is used as the error message.
/// If called with more than one argument, the first is a `format!()` format string,
/// and the remainder are the values to format.
///
/// This macro wraps the [`Exception::molt_err`](types/struct.Exception.html#method.molt_err)
/// method.
///
/// # Examples
///
/// ```
/// use molt_forked::*;
///
/// // Return a simple error message
/// fn err1() -> MoltResult {
///     // ...
///     molt_err!("error message")
/// }
///
/// let result = err1();
/// assert!(result.is_err());
///
/// let exception = result.err().unwrap();
/// assert!(exception.is_error());
/// assert_eq!(exception.value(), "error message".into());
///
/// // Return a formatted error
/// fn err2() -> MoltResult {
///    // ...
///    molt_err!("invalid value: {}", 17)
/// }
///
/// let result = err2();
/// assert!(result.is_err());
///
/// let exception = result.err().unwrap();
/// assert!(exception.is_error());
/// assert_eq!(exception.value(), "invalid value: 17".into());
/// ```
#[macro_export]
macro_rules! molt_err {
    ($arg:expr) => (
        Err($crate::Exception::molt_err($crate::Value::from($arg)))
    );
    ($($arg:tt)*) => (
        Err($crate::Exception::molt_err($crate::Value::from(format!($($arg)*))))
    )
}

#[macro_export]
macro_rules! molt_err_help {
    ($arg:expr) => {{
      let mut e = $crate::Exception::molt_err($crate::Value::from($arg));
      e.to_help();
      Err(e)
    }};
    ($($arg:tt)*) => {{
      let mut e = $crate::Exception::molt_err($crate::Value::from(format!($($arg)*)));
      e.to_help();
      Err(e)
    }}
}

/// Returns an `Error` `MoltResult` with a specific error code.  The error message is formatted
/// as with `format!()`.
///
/// The macro requires two or more arguments.  The first argument is always the error code.
/// If called with two arguments, the second is the error message.
/// If called with more than two arguments, the second is a `format!()` format string and
/// the remainder are the values to format.
///
/// This macro wraps
/// the [`Exception::molt_err2`](types/struct.Exception.html#method.molt_err2)
/// method.
///
/// # Examples
///
/// ```
/// use molt_forked::*;
///
/// // Throw a simple error
/// fn throw1() -> MoltResult {
///     // ...
///     molt_throw!("MYCODE", "error message")
/// }
///
/// let result = throw1();
/// assert!(result.is_err());
///
/// let exception = result.err().unwrap();
/// assert!(exception.is_error());
/// assert_eq!(exception.value(), "error message".into());
/// assert_eq!(exception.error_code(), "MYCODE".into());
///
/// // Return a formatted error
/// fn throw2() -> MoltResult {
///    // ...
///    molt_throw!("MYCODE", "invalid value: {}", 17)
/// }
///
/// let result = throw2();
/// assert!(result.is_err());
///
/// let exception = result.err().unwrap();
/// assert!(exception.is_error());
/// assert_eq!(exception.value(), "invalid value: 17".into());
/// assert_eq!(exception.error_code(), "MYCODE".into());
/// ```
#[macro_export]
macro_rules! molt_throw {
    ($code:expr, $msg:expr) => (
        Err($crate::Exception::molt_err2($crate::Value::from($code), $crate::Value::from($msg)))
    );
    ($code:expr, $($arg:tt)*) => (
        Err($crate::Exception::molt_err2($crate::Value::from($code), $crate::Value::from(format!($($arg)*))))
    )
}

/// Generates a statically dispatched Molt ensemble command.
///
/// Each entry contains a string-literal command name, its handler, and a string-literal
/// usage description. The help text is aligned at compile time using Unicode terminal
/// display widths. `subc` is the argument index containing the subcommand; it is normally
/// `1`, and can be larger for nested ensembles.
///
/// ```
/// use molt_forked::{gen_subcommand, molt_ok, Interp, MoltResult, Value};
///
/// fn show(_interp: &mut Interp<()>, _argv: &[Value]) -> MoltResult {
///     molt_ok!("shown")
/// }
///
/// let command = gen_subcommand!(
///     (),
///     1,
///     [("show", show, "show the current value")],
/// );
/// let mut interp = Interp::default();
/// assert_eq!(
///     command(&mut interp, &["item".into(), "show".into()])?.as_str(),
///     "shown"
/// );
/// # Ok::<(), molt_forked::Exception>(())
/// ```
///
/// Duplicate and reserved names are rejected at the name literal:
///
/// ```compile_fail
/// # use molt_forked::{gen_subcommand, Interp, MoltResult, Value};
/// # fn handler(_: &mut Interp<()>, _: &[Value]) -> MoltResult { unreachable!() }
/// let _ = gen_subcommand!(
///     (),
///     1,
///     [("same", handler, "first"), ("same", handler, "second")],
/// );
/// ```
///
/// ```compile_fail
/// # use molt_forked::{gen_subcommand, Interp, MoltResult, Value};
/// # fn handler(_: &mut Interp<()>, _: &[Value]) -> MoltResult { unreachable!() }
/// let _ = gen_subcommand!((), 1, [("-help", handler, "collision")]);
/// ```
///
/// The pre-0.5 four-field syntax is intentionally unsupported:
///
/// ```compile_fail
/// # use molt_forked::{gen_subcommand, Interp, MoltResult, Value};
/// # fn handler(_: &mut Interp<()>, _: &[Value]) -> MoltResult { unreachable!() }
/// let _ = gen_subcommand!((), 1, [("same", handler, 12, "old padding")]);
/// ```
#[macro_export]
macro_rules! gen_subcommand {
  ($ctx_type:ty, $subc:expr, [ $( ($cmd_name:literal, $cmd_func:expr, $cmd_help:literal $(,)?) ),* $(,)?] $(,)?) => {
    {
      |interp: &mut $crate::prelude::Interp<$ctx_type>, argv: &[$crate::prelude::Value]| -> $crate::prelude::MoltResult {
        $crate::check_args($subc, argv, $subc + 1, 0, "subcommand ?arg ...?")?;
        let sub_name = argv[$subc].as_str();
        const HELP_MSG: &str = $crate::__private::format_subcommand_help!([
          $(($cmd_name, $cmd_help),)*
        ]);
        match sub_name {
          $(
            $cmd_name => $cmd_func(interp, argv),
          )*
          "-help" => {
            let command_prefix = $crate::__private::list_to_string(&argv[0..$subc]);
            $crate::molt_ok!("usage of {}:\n{}", command_prefix, HELP_MSG)
          },
          _ => {
            let command_prefix = $crate::__private::list_to_string(&argv[0..$subc]);
            $crate::molt_err_help!("unknown subcommand in \"{} {}\", usage:\n{}", command_prefix, sub_name, HELP_MSG)
          },
        }
      }
    }
  }
}

/// Generates the interpreter's static top-level command dispatcher and help text.
///
/// Native entries are `(name_constant, handler)` pairs. Application commands are
/// `(name_literal, handler, help_literal)` triples. Names remain in declaration order;
/// dispatch and help generation require no runtime lookup table. Application names that
/// collide with Tcl built-ins are rejected at compile time.
///
/// ```compile_fail
/// # use molt_forked::{gen_command, Interp, MoltResult, Value};
/// # fn handler(_: &mut Interp<()>, _: &[Value]) -> MoltResult { unreachable!() }
/// let _ = gen_command!((), [], [("set", handler, "reserved")]);
/// ```
#[macro_export]
macro_rules! gen_command {
  ($ctx_type:ty, [ $( ($native_name:tt, $native_func:expr $(,)?) ),* $(,)?], [ $( ($embedded_name:literal, $embedded_func:expr, $embedded_help:literal $(,)?) ),* $(,)?] $(,)?) => {
    $crate::prelude::CommandSet::new(
      {fn f(name: &str, interp: &mut $crate::prelude::Interp<$ctx_type>, argv: &[$crate::prelude::Value]) -> $crate::prelude::MoltResult {
        const HELP_MSG: &str = $crate::__private::format_command_help!([
          $(($embedded_name, $embedded_help),)*
        ]);
        match name {
          "help" => {
            if let Some(v)= argv.get(1){
              if v.as_str()=="-all"{
                let proc_command_names = interp.proc_command_names();
                if proc_command_names.is_empty(){
                  return $crate::molt_ok!("usage of {}:\nbuiltins:\n  {}\n{}:\n{}", interp.name(),interp.native_command_names(),interp.name(),HELP_MSG);
                }else{
                  return $crate::molt_ok!("usage of {}:\nbuiltins:\n  {}\n{}:\n{}\nprocedure:\n  {}", interp.name(),interp.native_command_names(),interp.name(),HELP_MSG,proc_command_names);
                }
              }
            }
            $crate::molt_ok!("usage of {}:\n{}",interp.name(),HELP_MSG)},
          // NOTICE: Extra native commands
          $(
            $native_name => $native_func(interp, argv),
          )*
          // NOTICE: Embedded commands
          $(
            $embedded_name => $embedded_func(interp, argv),
          )*
          // NOTICE: Standard and proc commands
          other => {
            if let Some(result) = $crate::__private::execute_builtin(other, interp, argv) {
              result
            } else if let Some(result) = interp.try_execute_proc(other, argv) {
              result
            } else {
              let proc_command_names = interp.proc_command_names();
              if proc_command_names.is_empty(){
                $crate::molt_err_help!("unknown command \"{}\", valid commands:\nbuiltins:\n  {}\n{}:\n{}", name,interp.native_command_names(),interp.name(),HELP_MSG)
              }else{
                $crate::molt_err_help!("unknown command \"{}\", valid commands:\nbuiltins:\n  {}\n{}:\n{}\nprocedure:\n  {}", name,interp.native_command_names(),interp.name(),HELP_MSG,proc_command_names)
              }
            }
          }
        }
      }
      f as fn(&str, &mut $crate::prelude::Interp<$ctx_type>, &[$crate::prelude::Value]) -> $crate::prelude::MoltResult
      },
      {fn f(name: &str, interp: &$crate::prelude::Interp<$ctx_type>) -> Option<$crate::prelude::CommandKind> {
        if $crate::__private::is_builtin(name, interp.standard_library()) {
          return Some($crate::prelude::CommandKind::Native);
        }
        match name {
          $(
            $native_name => Some($crate::prelude::CommandKind::Native),
          )*
          $(
            $embedded_name => Some($crate::prelude::CommandKind::Embedded),
          )*
          other => {
            if interp.has_proc(other) {
              Some($crate::prelude::CommandKind::Proc)
            } else {
              None
            }
          }
        }
      }
      f as fn(&str, &$crate::prelude::Interp<$ctx_type>) -> Option<$crate::prelude::CommandKind>
      },
      &[
        $(
            $native_name,
        )*
      ],
      &[
        $(
          $embedded_name,
        )*
      ]
    )
  };
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_molt_ok() {
        let result: MoltResult = molt_ok!();
        assert_eq!(Ok(Value::empty()), result);

        let result: MoltResult = molt_ok!(5);
        assert_eq!(Ok(Value::from(5)), result);

        let result: MoltResult = molt_ok!("Five");
        assert_eq!(Ok(Value::from("Five")), result);

        let result: MoltResult = molt_ok!("The answer is {}.", 5);
        assert_eq!(Ok(Value::from("The answer is 5.")), result);
    }

    #[test]
    fn test_molt_except() {
        check_err(Err(molt_except!("error message")), "error message");
        check_err(Err(molt_except!("error {}", 5)), "error 5");
    }

    #[test]
    fn test_molt_err() {
        check_err(molt_err!("error message"), "error message");
        check_err(molt_err!("error {}", 5), "error 5");
    }

    #[test]
    fn test_molt_throw() {
        check_throw(molt_throw!("MYERR", "error message"), "MYERR", "error message");
        check_throw(molt_throw!("MYERR", "error {}", 5), "MYERR", "error 5");
    }

    fn check_err(result: MoltResult, msg: &str) -> bool {
        match result {
            Err(exception) => exception.is_error() && exception.value() == msg.into(),
            _ => false,
        }
    }

    fn check_throw(result: MoltResult, code: &str, msg: &str) -> bool {
        match result {
            Err(exception) => {
                exception.is_error()
                    && exception.value() == msg.into()
                    && exception.error_code() == code.into()
            }
            _ => false,
        }
    }
}
