//! # Standard Molt Command Definitions
//!
//! This module defines the standard Molt commands.

use crate::{
    dict::{dict_new, dict_path_insert, dict_path_remove, list_to_dict},
    interp::Interp,
    list::list_to_string,
    types::*,
    util, *,
};
#[cfg(feature = "full")]
use crate::{eval_ptr::EvalPtr, tokenizer::Tokenizer};
#[cfg(not(feature = "wasm"))]
use std::time::Instant;
use std::{borrow::Cow, fs, rc::Rc};
#[cfg(feature = "wasm")]
use web_time::Instant;

pub const _ASSERT_EQ: &str = "assert_eq";
pub const _SOURCE: &str = "source";
pub const _EXIT: &str = "exit";
pub const _PARSE: &str = "parse";

macro_rules! builtin_library {
    (slim) => {
        StandardLibrary::Slim
    };
    (full) => {
        StandardLibrary::Full
    };
}

macro_rules! execute_builtin_entry {
    (slim, $interp:ident, $argv:ident, $handler:ident) => {
        Some($handler($interp, $argv))
    };
    (full, $interp:ident, $argv:ident, $handler:ident) => {
        if $interp.standard_library() == StandardLibrary::Full {
            Some($handler($interp, $argv))
        } else {
            None
        }
    };
}

macro_rules! builtin_entry_exists {
    (slim, $library:ident) => {
        true
    };
    (full, $library:ident) => {
        $library == StandardLibrary::Full
    };
}

#[derive(Clone, Copy)]
struct BuiltinCommand {
    name: &'static str,
    library: StandardLibrary,
}

macro_rules! define_builtin_commands {
    ($( $(#[$meta:meta])* $library:ident $name:literal => $handler:ident ),+ $(,)?) => {
        const BUILTIN_COMMANDS: &[BuiltinCommand] = &[
            $($(#[$meta])* BuiltinCommand {
                name: $name,
                library: builtin_library!($library),
            },)+
        ];

        /// Iterates over the standard commands enabled for a library profile.
        pub(crate) fn builtin_command_names(
            library: StandardLibrary,
        ) -> impl Iterator<Item = &'static str> {
            BUILTIN_COMMANDS.iter().filter_map(move |command| {
                (command.library == StandardLibrary::Slim
                    || library == StandardLibrary::Full)
                    .then_some(command.name)
            })
        }

        /// Dispatches a standard command without allocating a registry or hashing its name.
        #[doc(hidden)]
        #[inline(always)]
        pub fn execute_builtin<Ctx>(
            name: &str,
            interp: &mut Interp<Ctx>,
            argv: &[Value],
        ) -> Option<MoltResult> {
            match name {
                $($(#[$meta])* $name => execute_builtin_entry!($library, interp, argv, $handler),)+
                _ => None,
            }
        }

        /// Returns whether a name belongs to the static standard command set.
        #[doc(hidden)]
        #[inline(always)]
        pub fn is_builtin(name: &str, _library: StandardLibrary) -> bool {
            match name {
                $($(#[$meta])* $name => builtin_entry_exists!($library, _library),)+
                _ => false,
            }
        }
    };
}

define_builtin_commands! {
    slim "append" => cmd_append,
    #[cfg(feature = "full")]
    full "apply" => cmd_apply,
    slim "array" => cmd_array,
    slim "assert_eq" => cmd_assert_eq,
    slim "break" => cmd_break,
    slim "catch" => cmd_catch,
    slim "continue" => cmd_continue,
    #[cfg(feature = "full")]
    full "concat" => cmd_concat,
    slim "dict" => cmd_dict,
    slim "error" => cmd_error,
    #[cfg(feature = "full")]
    full "eval" => cmd_eval,
    slim "expr" => cmd_expr,
    slim "for" => cmd_for,
    slim "foreach" => cmd_foreach,
    slim "global" => cmd_global,
    slim "if" => cmd_if,
    slim "incr" => cmd_incr,
    slim "info" => cmd_info,
    slim "join" => cmd_join,
    slim "lappend" => cmd_lappend,
    #[cfg(feature = "full")]
    full "lassign" => cmd_lassign,
    slim "lindex" => cmd_lindex,
    #[cfg(feature = "full")]
    full "linsert" => cmd_linsert,
    slim "list" => cmd_list,
    slim "llength" => cmd_llength,
    #[cfg(feature = "full")]
    full "lmap" => cmd_lmap,
    #[cfg(feature = "full")]
    full "lrange" => cmd_lrange,
    #[cfg(feature = "full")]
    full "lrepeat" => cmd_lrepeat,
    #[cfg(feature = "full")]
    full "lreplace" => cmd_lreplace,
    #[cfg(feature = "full")]
    full "lreverse" => cmd_lreverse,
    slim "proc" => cmd_proc,
    slim "puts" => cmd_puts,
    slim "rename" => cmd_rename,
    slim "return" => cmd_return,
    slim "set" => cmd_set,
    #[cfg(feature = "full")]
    full "split" => cmd_split,
    slim "string" => cmd_string,
    #[cfg(feature = "full")]
    full "subst" => cmd_subst,
    #[cfg(feature = "full")]
    full "switch" => cmd_switch,
    slim "throw" => cmd_throw,
    slim "time" => cmd_time,
    #[cfg(feature = "full")]
    full "try" => cmd_try,
    slim "unset" => cmd_unset,
    #[cfg(feature = "full")]
    full "uplevel" => cmd_uplevel,
    #[cfg(feature = "full")]
    full "upvar" => cmd_upvar,
    slim "while" => cmd_while,
}

/// # apply lambdaExpr ?arg ...?
#[cfg(feature = "full")]
pub fn cmd_apply<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 0, "lambdaExpr ?arg ...?")?;
    let lambda = argv[1].as_list()?;
    if !(2..=3).contains(&lambda.len()) {
        return molt_err!("can't interpret \"{}\" as a lambda expression", argv[1]);
    }
    if lambda.len() == 3 && lambda[2].as_str() != "::" {
        return molt_err!("namespace \"{}\" not found", lambda[2]);
    }
    let parameters = lambda[0].as_list()?;
    validate_parameters(&parameters)?;
    let mut call = Vec::with_capacity(argv.len() - 1);
    call.push(argv[0].clone());
    call.extend(argv[2..].iter().cloned());
    interp.execute_anonymous(&parameters, &lambda[1], &call)
}

fn validate_parameters(parameters: &[Value]) -> Result<(), Exception> {
    for parameter in parameters {
        let fields = parameter.as_list()?;
        if fields.is_empty() {
            return molt_err!("argument with no name");
        }
        if fields.len() > 2 {
            return molt_err!("too many fields in argument specifier \"{}\"", parameter);
        }
    }
    Ok(())
}

/// # append *varName* ?*value* ...?
///
/// Appends one or more strings to a variable.
/// See molt-book for full semantics.
pub fn cmd_append<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 0, "varName ?value value ...?")?;

    // FIRST, get the value of the variable.  If the variable is undefined,
    // start with the empty string.
    let mut new_string: String = interp
        .var(&argv[1])
        .map(|val| val.to_string())
        .unwrap_or_else(|_| String::new());

    // NEXT, append the remaining values to the string.
    new_string.reserve(argv[2..].iter().map(|item| item.as_str().len()).sum());
    for item in &argv[2..] {
        new_string.push_str(item.as_str());
    }

    // NEXT, save and return the new value.
    interp.set_var_return(&argv[1], new_string.into())
}

/// # array *subcommand* ?*arg*...?
///
/// <https://www.tcl.tk/man/tcl8.6/TclCmd/array.htm>
pub fn cmd_array<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    let f = gen_subcommand!(
        Ctx,
        1,
        [
            ("exists", cmd_array_exists, "array exists arrayName"),
            ("get", cmd_array_get, "array get arrayName ?pattern?"),
            ("names", cmd_array_names, "array names arrayName ?mode? ?pattern?"),
            ("set", cmd_array_set, "array set arrayName list"),
            ("size", cmd_array_size, "array size arrayName"),
            ("unset", cmd_array_unset, "array unset arrayName ?pattern?"),
        ],
    );
    f(interp, argv)
}

/// # array exists arrayName
pub fn cmd_array_exists<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 3, "arrayName")?;
    molt_ok!(Value::from(interp.array_exists(argv[2].as_str())))
}

/// # array names arrayName ?mode? ?pattern?
pub fn cmd_array_names<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 5, "arrayName ?mode? ?pattern?")?;
    let names = interp.array_names(argv[2].as_str());
    if argv.len() == 3 {
        return molt_ok!(names);
    }
    let (mode, pattern) = if argv.len() == 4 {
        ("-glob", argv[3].as_str())
    } else {
        (argv[3].as_str(), argv[4].as_str())
    };
    let filtered = match mode {
        "-exact" => names.into_iter().filter(|name| name.as_str() == pattern).collect(),
        "-glob" => filter_glob(names, pattern),
        "-regexp" => return molt_err!("regular expression matching is not available"),
        _ => {
            return molt_err!(
                "bad option \"{}\": must be -exact, -glob, or -regexp",
                mode
            );
        }
    };
    molt_ok!(filtered)
}

/// # array get arrayName ?pattern?
pub fn cmd_array_get<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 4, "arrayName ?pattern?")?;
    let values = interp.array_get(argv[2].as_str());
    let Some(pattern) = argv.get(3).map(Value::as_str) else {
        return molt_ok!(values);
    };
    let mut filtered = Vec::with_capacity(values.len());
    for pair in values.chunks_exact(2) {
        if util::glob_match(pattern, pair[0].as_str(), false) {
            filtered.extend_from_slice(pair);
        }
    }
    molt_ok!(filtered)
}

/// # parse *script*
///
/// A command for parsing an arbitrary script and outputting the parsed form.
/// This is an undocumented debugging aid.  The output can be greatly improved.
pub fn cmd_parse<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 2, "script")?;

    let script = &argv[1];

    molt_ok!(format!("{:?}", parser::parse(script.as_str())?))
}

/// # array set arrayName list
pub fn cmd_array_set<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 4, "arrayName list")?;

    // This odd little dance provides the same semantics as Standard TCL.  If the
    // given var_name has an index, the array is created (if it didn't exist)
    // but no data is added to it, and the command returns an error.
    let var_name = argv[2].as_var_name();

    if var_name.index().is_none() {
        interp.array_set(var_name.name(), &argv[3].as_list()?)
    } else {
        // This line will create the array if it doesn't exist, and throw an error if the
        // named variable exists but isn't an array.  This is a little wacky, but it's
        // what TCL 8.6 does.
        interp.array_set(var_name.name(), &Value::empty().as_list()?)?;

        // And this line throws an error because the full name the caller specified is an
        // element, not the array itself.
        molt_err!("can't set \"{}\": variable isn't array", &argv[2])
    }
}

/// # array size arrayName
pub fn cmd_array_size<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 3, "arrayName")?;
    molt_ok!(Value::from(interp.array_size(argv[2].as_str()) as MoltInt))
}

/// # array unset arrayName ?*pattern*?
pub fn cmd_array_unset<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 4, "arrayName ?pattern?")?;

    if argv.len() == 3 {
        interp.array_unset(argv[2].as_str());
    } else {
        let array = argv[2].as_str();
        let pattern = argv[3].as_str();
        let names = interp.array_names(array);
        for name in names {
            if util::glob_match(pattern, name.as_str(), false) {
                interp.unset_element(array, name.as_str());
            }
        }
    }
    molt_ok!()
}

/// assert_eq received, expected
///
/// Asserts that two values have identical string representations.
/// See molt-book for full semantics.
pub fn cmd_assert_eq<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 3, 3, "received expected")?;

    if argv[1] == argv[2] {
        molt_ok!()
    } else {
        molt_err!("assertion failed: received \"{}\", expected \"{}\".", argv[1], argv[2])
    }
}

/// # break
///
/// Breaks a loops.
/// See molt-book for full semantics.
pub fn cmd_break<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 1, 1, "")?;

    Err(Exception::molt_break())
}

/// catch script ?resultVarName? ?optionsVarName?
///
/// Executes a script, returning the result code.  If the resultVarName is given, the result
/// of executing the script is returned in it.  The result code is returned as an integer,
/// 0=Ok, 1=Error, 2=Return, 3=Break, 4=Continue.
pub fn cmd_catch<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 4, "script ?resultVarName? ?optionsVarName?")?;

    // If the script called `return x`, should get Return, -level 1, -code Okay here
    let result = interp.eval_value(&argv[1]);

    let (code, value) = match &result {
        Ok(val) => (0, val.clone()),
        Err(exception) => match exception.code() {
            ResultCode::Okay => unreachable!(), // Should not be reachable here.
            ResultCode::Error => (1, exception.value()),
            ResultCode::Return => (2, exception.value()),
            ResultCode::Break => (3, exception.value()),
            ResultCode::Continue => (4, exception.value()),
            ResultCode::Other(num) => (num, exception.value()),
        },
    };

    if argv.len() >= 3 {
        interp.set_var(&argv[2], value)?;
    }

    if argv.len() == 4 {
        interp.set_var(&argv[3], interp.return_options(&result))?;
    }

    Ok(Value::from(code))
}

/// # continue
///
/// Continues with the next iteration of the inmost loop.
pub fn cmd_continue<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 1, 1, "")?;

    Err(Exception::molt_continue())
}

/// # concat ?arg ...?
///
/// Concatenates zero or more Tcl lists into one canonical list.
#[cfg(feature = "full")]
pub fn cmd_concat<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    let capacity: usize = argv[1..]
        .iter()
        .filter_map(|value| value.try_as_str())
        .map(str::len)
        .sum();
    let mut values = Vec::with_capacity(capacity / 2);
    for value in &argv[1..] {
        values.extend(value.as_list()?.iter().cloned());
    }
    molt_ok!(values)
}

/// # dict *subcommand* ?*arg*...?
///
/// <https://www.tcl.tk/man/tcl8.6/TclCmd/dict.htm>
pub fn cmd_dict<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    let f = gen_subcommand!(
        Ctx,
        1,
        [
            ("append", cmd_dict_append, "dict append dictVarName key ?string ...?"),
            ("create", cmd_dict_new, "dict create ?key value ...?"),
            ("exists", cmd_dict_exists, "dict exists dictionary key ?key ...?"),
            ("get", cmd_dict_get, "dict get dictionary ?key ...?"),
            (
                "getwithdefault",
                cmd_dict_get_with_default,
                "dict getwithdefault dictionary key defaultValue"
            ),
            ("incr", cmd_dict_incr, "dict incr dictVarName key ?increment?"),
            ("keys", cmd_dict_keys, "dict keys dictionary ?pattern?"),
            ("lappend", cmd_dict_lappend, "dict lappend dictVarName key ?value ...?"),
            ("merge", cmd_dict_merge, "dict merge ?dictionary ...?"),
            ("remove", cmd_dict_remove, "dict remove dictionary ?key ...?"),
            ("replace", cmd_dict_replace, "dict replace dictionary ?key value ...?"),
            ("set", cmd_dict_set, "dict set dictVarName key ?key ...? value"),
            ("size", cmd_dict_size, "dict size dictionary"),
            ("unset", cmd_dict_unset, "dict unset dictVarName key ?key ...?"),
            ("values", cmd_dict_values, "dict values dictionary ?pattern?"),
        ],
    );
    f(interp, argv)
}

/// # dict append dictVarName key ?string ...?
fn cmd_dict_append<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 0, "dictVarName key ?string ...?")?;
    let mut dict = match interp.var(&argv[2]) {
        Ok(value) => (*value.as_dict()?).clone(),
        Err(_) => dict_new(),
    };
    let mut value = dict.get(&argv[3]).map_or_else(String::new, ToString::to_string);
    value.reserve(argv[4..].iter().map(|item| item.as_str().len()).sum());
    for item in &argv[4..] {
        value.push_str(item.as_str());
    }
    dict.insert(argv[3].clone(), Value::from(value));
    interp.set_var_return(&argv[2], Value::from(dict))
}

/// # dict create ?key value ...?
fn cmd_dict_new<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    // FIRST, we need an even number of arguments.
    if !argv.len().is_multiple_of(2) {
        return molt_err!(
            "wrong # args: should be \"{} {}\"",
            list_to_string(&argv[0..2]),
            "?key value?"
        );
    }

    // NEXT, return the value.
    if argv.len() > 2 {
        molt_ok!(Value::from(list_to_dict(&argv[2..])))
    } else {
        molt_ok!(Value::from(dict_new()))
    }
}

/// # dict exists *dictionary* key ?*key* ...?
fn cmd_dict_exists<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 0, "dictionary key ?key ...?")?;

    let mut value: Value = argv[2].clone();
    let indices = &argv[3..];

    for index in indices {
        if let Ok(dict) = value.as_dict() {
            if let Some(val) = dict.get(index) {
                value = val.clone();
            } else {
                return molt_ok!(false);
            }
        } else {
            return molt_ok!(false);
        }
    }

    molt_ok!(true)
}

/// # dict get *dictionary* ?*key* ...?
fn cmd_dict_get<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 0, "dictionary ?key ...?")?;

    let mut value: Value = argv[2].clone();
    let indices = &argv[3..];

    for index in indices {
        let dict = value.as_dict()?;

        if let Some(val) = dict.get(index) {
            value = val.clone();
        } else {
            return molt_err!("key \"{}\" not known in dictionary", index);
        }
    }

    molt_ok!(value)
}

/// # dict getwithdefault dictionary key defaultValue
fn cmd_dict_get_with_default<Ctx>(
    _interp: &mut Interp<Ctx>,
    argv: &[Value],
) -> MoltResult {
    check_args(2, argv, 5, 5, "dictionary key defaultValue")?;
    let dict = argv[2].as_dict()?;
    molt_ok!(dict.get(&argv[3]).unwrap_or(&argv[4]).clone())
}

/// # dict incr dictVarName key ?increment?
fn cmd_dict_incr<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 5, "dictVarName key ?increment?")?;
    let mut dict = match interp.var(&argv[2]) {
        Ok(value) => (*value.as_dict()?).clone(),
        Err(_) => dict_new(),
    };
    let value = add_integer_values(dict.get(&argv[3]), argv.get(4))?;
    dict.insert(argv[3].clone(), value);
    interp.set_var_return(&argv[2], Value::from(dict))
}

/// # dict keys *dictionary* ?*pattern*?
fn cmd_dict_keys<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 4, "dictionary ?pattern?")?;

    let dict = argv[2].as_dict()?;
    let pattern = argv.get(3).map(Value::as_str);
    let keys: MoltList = dict
        .keys()
        .filter(|key| {
            pattern.is_none_or(|pattern| util::glob_match(pattern, key.as_str(), false))
        })
        .cloned()
        .collect();
    molt_ok!(keys)
}

/// # dict lappend dictVarName key ?value ...?
fn cmd_dict_lappend<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 0, "dictVarName key ?value ...?")?;
    let mut dict = match interp.var(&argv[2]) {
        Ok(value) => (*value.as_dict()?).clone(),
        Err(_) => dict_new(),
    };
    let mut list = dict.get(&argv[3]).map_or_else(|| Ok(Vec::new()), Value::to_list)?;
    list.extend(argv[4..].iter().cloned());
    dict.insert(argv[3].clone(), Value::from(list));
    interp.set_var_return(&argv[2], Value::from(dict))
}

/// # dict merge ?dictionary ...?
fn cmd_dict_merge<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    let mut output = dict_new();
    for value in &argv[2..] {
        for (key, value) in value.as_dict()?.iter() {
            output.insert(key.clone(), value.clone());
        }
    }
    molt_ok!(output)
}

/// # dict remove *dictionary* ?*key* ...?
fn cmd_dict_remove<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 0, "dictionary ?key ...?")?;

    // FIRST, get and clone the dictionary, so we can modify it.
    let mut dict = (*argv[2].as_dict()?).clone();

    // NEXT, remove the given keys.
    for key in &argv[3..] {
        // shift_remove preserves the order of the keys.
        dict.shift_remove(key);
    }

    // NEXT, return it as a new Value.
    molt_ok!(dict)
}

/// # dict replace dictionary ?key value ...?
fn cmd_dict_replace<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 0, "dictionary ?key value ...?")?;
    if argv.len().is_multiple_of(2) {
        return molt_err!(
            "wrong # args: should be \"dict replace dictionary ?key value ...?\""
        );
    }
    let mut dict = (*argv[2].as_dict()?).clone();
    for pair in argv[3..].chunks_exact(2) {
        dict.insert(pair[0].clone(), pair[1].clone());
    }
    molt_ok!(dict)
}

/// # dict set *dictVarName* *key* ?*key* ...? *value*
fn cmd_dict_set<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 5, 0, "dictVarName key ?key ...? value")?;

    let value = &argv[argv.len() - 1];
    let keys = &argv[3..(argv.len() - 1)];

    if let Ok(old_dict_val) = interp.var(&argv[2]) {
        interp.set_var_return(&argv[2], dict_path_insert(&old_dict_val, keys, value)?)
    } else {
        let new_val = Value::from(dict_new());
        interp.set_var_return(&argv[2], dict_path_insert(&new_val, keys, value)?)
    }
}

/// # dict size *dictionary*
fn cmd_dict_size<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 3, "dictionary")?;

    let dict = argv[2].as_dict()?;
    molt_ok!(dict.len() as MoltInt)
}

/// # dict unset *dictVarName* *key* ?*key* ...?
fn cmd_dict_unset<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 0, "dictVarName key ?key ...?")?;

    let keys = &argv[3..];

    if let Ok(old_dict_val) = interp.var(&argv[2]) {
        interp.set_var_return(&argv[2], dict_path_remove(&old_dict_val, keys)?)
    } else {
        let new_val = Value::from(dict_new());
        interp.set_var_return(&argv[2], dict_path_remove(&new_val, keys)?)
    }
}

/// # dict values *dictionary* ?*pattern*?
fn cmd_dict_values<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 4, "dictionary ?pattern?")?;

    let dict = argv[2].as_dict()?;
    let pattern = argv.get(3).map(Value::as_str);
    let values: MoltList = dict
        .values()
        .filter(|value| {
            pattern.is_none_or(|pattern| util::glob_match(pattern, value.as_str(), false))
        })
        .cloned()
        .collect();
    molt_ok!(values)
}

/// error *message* ?*info*? ?*code*?
///
/// Returns an error with the given message.
///
pub fn cmd_error<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 4, "message ?errorInfo? ?errorCode?")?;
    if argv.len() == 2 {
        molt_err!(argv[1].clone())
    } else {
        Err(Exception::molt_return_err(
            argv[1].clone(),
            0,
            argv.get(3).cloned(),
            argv.get(2).cloned(),
        ))
    }
}

/// # eval arg ?arg ...?
///
/// Concatenates its arguments as Tcl lists and evaluates the resulting script.
#[cfg(feature = "full")]
pub fn cmd_eval<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 0, "arg ?arg ...?")?;

    if argv.len() == 2 {
        return interp.eval_value(&argv[1]);
    }

    let mut words = Vec::new();
    for value in &argv[1..] {
        words.extend(value.as_list()?.iter().cloned());
    }
    interp.eval(&list_to_string(&words))
}

/// # exit ?*returnCode*?
///
/// Terminates the application by calling `std::process::exit()`.
/// If given, _returnCode_ must be an integer return code; if absent, it
/// defaults to 0.
pub fn cmd_exit<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 1, 2, "?returnCode?")?;

    let return_code: MoltInt = if argv.len() == 1 { 0 } else { argv[1].as_int()? };

    std::process::exit(return_code as i32)
}

/// # expr expr
///
/// Evaluates an expression and returns its result.
///
/// ## TCL Liens
///
/// See the Molt Book.
pub fn cmd_expr<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 0, "expr")?;

    if argv.len() == 2 {
        interp.expr(&argv[1])
    } else {
        let mut expression = String::new();
        for (index, value) in argv[1..].iter().enumerate() {
            if index != 0 {
                expression.push(' ');
            }
            expression.push_str(value.as_str());
        }
        interp.expr(&Value::from(expression))
    }
}

/// # for *start* *test* *next* *command*
///
/// A standard "for" loop.  start, next, and command are scripts; test is an expression
///
pub fn cmd_for<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 5, 5, "start test next command")?;

    let start = &argv[1];
    let test = &argv[2];
    let next = &argv[3];
    let command = &argv[4];

    // Start
    interp.eval_value(start)?;

    while interp.expr_bool(test)? {
        let result = interp.eval_value(command);

        if let Err(exception) = result {
            match exception.code() {
                ResultCode::Break => break,
                ResultCode::Continue => (),
                _ => return Err(exception),
            }
        }

        // Execute next script.  Break is allowed, but continue is not.
        let result = interp.eval_value(next);

        if let Err(exception) = result {
            match exception.code() {
                ResultCode::Break => break,
                ResultCode::Continue => {
                    return molt_err!("invoked \"continue\" outside of a loop");
                }
                _ => return Err(exception),
            }
        }
    }

    molt_ok!()
}

/// # foreach *varList* *list* ?*varList list* ...? *body*
pub fn cmd_foreach<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    foreach_impl::<_, false>(interp, argv)
}

/// # lmap *varList* *list* ?*varList list* ...? *body*
#[cfg(feature = "full")]
pub fn cmd_lmap<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    foreach_impl::<_, true>(interp, argv)
}

fn foreach_impl<Ctx, const COLLECT: bool>(
    interp: &mut Interp<Ctx>,
    argv: &[Value],
) -> MoltResult {
    check_args(1, argv, 4, 0, "varList list ?varList list ...? body")?;
    if !argv.len().is_multiple_of(2) {
        return molt_err!(
            "wrong # args: should be \"{} varList list ?varList list ...? body\"",
            argv[0]
        );
    }

    let mut groups: Vec<(Rc<MoltList>, Rc<MoltList>)> =
        Vec::with_capacity((argv.len() - 2) / 2);
    let mut iterations = 0;
    for pair in argv[1..argv.len() - 1].chunks_exact(2) {
        let variables = pair[0].as_list()?;
        if variables.is_empty() {
            return molt_err!("{} varlist is empty", argv[0]);
        }
        let values = pair[1].as_list()?;
        iterations = iterations.max(values.len().div_ceil(variables.len()));
        groups.push((variables, values));
    }

    let body = argv.last().expect("argument count checked");
    let mut output = Vec::with_capacity(if COLLECT { iterations } else { 0 });
    'iterations: for iteration in 0..iterations {
        for (variables, values) in &groups {
            let offset = iteration * variables.len();
            for (slot, variable) in variables.iter().enumerate() {
                interp.set_var(
                    variable,
                    values.get(offset + slot).cloned().unwrap_or_else(Value::empty),
                )?;
            }
        }
        match interp.eval_value(body) {
            Ok(value) => {
                if COLLECT {
                    output.push(value);
                }
            }
            Err(exception) => match exception.code() {
                ResultCode::Break => break 'iterations,
                ResultCode::Continue => continue 'iterations,
                _ => return Err(exception),
            },
        }
    }
    if COLLECT {
        molt_ok!(output)
    } else {
        molt_ok!()
    }
}

/// # global ?*varName* ...?
///
/// Appends any number of values to a variable's value, which need not
/// initially exist.
pub fn cmd_global<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    // Accepts any number of arguments

    // FIRST, if we're at the global scope this is a no-op.
    if interp.scope_level() > 0 {
        for name in &argv[1..] {
            // TODO: Should upvar take the name as a Value?
            interp.upvar(0, name.as_str());
        }
    }
    molt_ok!()
}

#[derive(Eq, PartialEq, Debug)]
enum IfWants {
    Expr,
    ThenBody,
    SkipThenClause,
    ElseClause,
    ElseBody,
}

/// # if *expr* ?then? *script* elseif *expr* ?then? *script* ... ?else? ?*script*?
///
/// Standard conditional.  Returns the value of the selected script (or
/// "" if there is no else body and the none of the previous branches were selected).
///
/// # TCL Liens
///
/// * Because we don't yet have an expression parser, the *expr* arguments are evaluated as
///   scripts that must return a boolean value.
pub fn cmd_if<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    let mut argi = 1;
    let mut wants = IfWants::Expr;

    while argi < argv.len() {
        match wants {
            IfWants::Expr => {
                wants = if interp.expr_bool(&argv[argi])? {
                    IfWants::ThenBody
                } else {
                    IfWants::SkipThenClause
                };
            }
            IfWants::ThenBody => {
                if argv[argi].as_str() == "then" {
                    argi += 1;
                }

                if argi < argv.len() {
                    return interp.eval_value(&argv[argi]);
                } else {
                    break;
                }
            }
            IfWants::SkipThenClause => {
                if argv[argi].as_str() == "then" {
                    argi += 1;
                }

                if argi < argv.len() {
                    argi += 1;
                    wants = IfWants::ElseClause;
                }
                continue;
            }
            IfWants::ElseClause => {
                if argv[argi].as_str() == "elseif" {
                    wants = IfWants::Expr;
                } else {
                    wants = IfWants::ElseBody;
                    continue;
                }
            }
            IfWants::ElseBody => {
                if argv[argi].as_str() == "else" {
                    argi += 1;

                    // If "else" appears, then the else body is required.
                    if argi == argv.len() {
                        return molt_err!(
                            "wrong # args: no script following after \"{}\" argument",
                            argv[argi - 1]
                        );
                    }
                }

                if argi < argv.len() {
                    return interp.eval_value(&argv[argi]);
                } else {
                    break;
                }
            }
        }

        argi += 1;
    }

    if argi < argv.len() {
        molt_err!("wrong # args: extra words after \"else\" clause in \"if\" command")
    } else if wants == IfWants::Expr {
        molt_err!("wrong # args: no expression after \"{}\" argument", argv[argi - 1])
    } else if wants == IfWants::ThenBody || wants == IfWants::SkipThenClause {
        molt_err!(
            "wrong # args: no script following after \"{}\" argument",
            argv[argi - 1]
        )
    } else {
        // Looking for ElseBody, but there doesn't need to be one.
        molt_ok!() // temp
    }
}

/// # incr *varName* ?*increment* ...?
///
/// Increments an integer variable by a value.
pub fn cmd_incr<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 3, "varName ?increment?")?;
    let current = interp.optional_var(&argv[1])?;
    let new_value = add_integer_values(current.as_ref(), argv.get(2))?;
    interp.set_var_return(&argv[1], new_value)
}

#[cfg(feature = "full")]
#[inline]
fn add_integer_values(current: Option<&Value>, increment: Option<&Value>) -> MoltResult {
    let small_current = current.map_or(Ok(0), Value::as_int);
    let small_increment = increment.map_or(Ok(1), Value::as_int);
    if let (Ok(current), Ok(increment)) = (small_current, small_increment) {
        if let Some(sum) = current.checked_add(increment) {
            return molt_ok!(sum);
        }
    }

    // Promote only invalid fixed-width combinations to the arbitrary-precision path. This
    // covers existing bignums and true i64 overflow without charging small integer updates.
    let current = match current {
        Some(value) => value.as_bignum()?,
        None => Rc::new(MoltBigInt::from(0)),
    };
    let increment = match increment {
        Some(value) => value.as_bignum()?,
        None => Rc::new(MoltBigInt::from(1)),
    };
    molt_ok!(current.as_ref() + increment.as_ref())
}

#[cfg(not(feature = "full"))]
#[inline]
fn add_integer_values(current: Option<&Value>, increment: Option<&Value>) -> MoltResult {
    let current = current.map_or(Ok(0), Value::as_int)?;
    let increment = increment.map_or(Ok(1), Value::as_int)?;
    current
        .checked_add(increment)
        .map(Value::from)
        .ok_or_else(|| Exception::molt_err("integer value too large to represent".into()))
}

/// # info *subcommand* ?*arg*...?
pub fn cmd_info<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    let f = gen_subcommand!(
        Ctx,
        1,
        [
            ("args", cmd_info_args, "info args procname"),
            ("body", cmd_info_body, "info body procname"),
            ("cmdtype", cmd_info_cmdtype, "info cmdtype command"),
            ("commands", cmd_info_commands, "info commands ?pattern?"),
            ("complete", cmd_info_complete, "info complete command"),
            ("default", cmd_info_default, "info default procname arg varname"),
            ("exists", cmd_info_exists, "info exists varName"),
            ("globals", cmd_info_globals, "info globals ?pattern?"),
            ("locals", cmd_info_locals, "info locals ?pattern?"),
            ("procs", cmd_info_procs, "info procs ?pattern?"),
            ("vars", cmd_info_vars, "info vars ?pattern?"),
        ],
    );
    f(interp, argv)
}

/// # info args *procname*
pub fn cmd_info_args<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 3, "procname")?;
    interp.proc_args(argv[2].as_str())
}

/// # info body *procname*
pub fn cmd_info_body<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 3, "procname")?;
    interp.proc_body(argv[2].as_str())
}

/// # info cmdtype *command*
pub fn cmd_info_cmdtype<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 3, "command")?;
    interp.command_type(argv[2].as_str())
}

/// # info commands ?*pattern*?
pub fn cmd_info_commands<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 2, 3, "?pattern?")?;
    molt_ok!(filter_optional_glob(interp.command_names(), argv.get(2)))
}

/// # info default *procname* *arg* *varname*
pub fn cmd_info_default<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 5, 5, "procname arg varname")?;

    if let Some(val) = interp.proc_default(argv[2].as_str(), argv[3].as_str())? {
        interp.set_var(&argv[4], val)?;
        molt_ok!(1)
    } else {
        interp.set_var(&argv[4], Value::empty())?;
        molt_ok!(0)
    }
}

/// # info exists *varname*
pub fn cmd_info_exists<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 3, "varname")?;
    Ok(interp.var_exists(&argv[2]).into())
}

/// # info complete *command*
pub fn cmd_info_complete<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 3, "command")?;
    molt_ok!(!crate::syntax::script_status(argv[2].as_str()).is_incomplete())
}

/// # info globals
/// TODO: Add glob matching as a feature, and provide optional pattern argument.
pub fn cmd_info_globals<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 2, 3, "?pattern?")?;
    molt_ok!(filter_optional_glob(interp.vars_in_global_scope(), argv.get(2)))
}

/// # info locals
/// TODO: Add glob matching as a feature, and provide optional pattern argument.
pub fn cmd_info_locals<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 2, 3, "?pattern?")?;
    molt_ok!(filter_optional_glob(interp.vars_in_local_scope(), argv.get(2)))
}

/// # info procs ?*pattern*?
pub fn cmd_info_procs<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 2, 3, "?pattern?")?;
    molt_ok!(filter_optional_glob(interp.proc_names(), argv.get(2)))
}

/// # info vars
/// TODO: Add glob matching as a feature, and provide optional pattern argument.
pub fn cmd_info_vars<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 2, 3, "?pattern?")?;
    molt_ok!(filter_optional_glob(interp.vars_in_scope(), argv.get(2)))
}

fn filter_optional_glob(values: MoltList, pattern: Option<&Value>) -> MoltList {
    match pattern {
        Some(pattern) => filter_glob(values, pattern.as_str()),
        None => values,
    }
}

fn filter_glob(values: MoltList, pattern: &str) -> MoltList {
    values
        .into_iter()
        .filter(|value| util::glob_match(pattern, value.as_str(), false))
        .collect()
}

/// # join *list* ?*joinString*?
///
/// Joins the elements of a list with a string.  The join string defaults to " ".
pub fn cmd_join<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 3, "list ?joinString?")?;

    let list = &argv[1].as_list()?;

    let join_string = if argv.len() == 3 { argv[2].as_str() } else { " " };
    let item_len: usize = list.iter().map(|value| value.as_str().len()).sum();
    let mut output = String::with_capacity(
        item_len + join_string.len().saturating_mul(list.len().saturating_sub(1)),
    );
    for (index, value) in list.iter().enumerate() {
        if index != 0 {
            output.push_str(join_string);
        }
        output.push_str(value.as_str());
    }

    molt_ok!(output)
}

/// # lappend *varName* ?*value* ...?
///
/// Appends any number of values to a variable's list value, which need not
/// initially exist.
pub fn cmd_lappend<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 0, "varName ?value ...?")?;

    let mut list = match interp.var(&argv[1]) {
        Ok(value) => value.to_list()?,
        Err(_) => Vec::new(),
    };

    let mut values = argv[2..].to_owned();
    list.append(&mut values);
    interp.set_var_return(&argv[1], Value::from(list))
}

/// # lassign list varName ?varName ...?
#[cfg(feature = "full")]
pub fn cmd_lassign<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 3, 0, "list varName ?varName ...?")?;

    let list = argv[1].as_list()?;
    for (offset, name) in argv[2..].iter().enumerate() {
        interp.set_var(name, list.get(offset).cloned().unwrap_or_else(Value::empty))?;
    }
    molt_ok!(Value::from(&list[usize::min(argv.len() - 2, list.len())..]))
}

/// # lindex *list* ?*index* ...?
///
/// Returns an element from the list, indexing into nested lists.
pub fn cmd_lindex<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 0, "list ?index ...?")?;

    if argv.len() != 3 {
        lindex_into(&argv[1], &argv[2..])
    } else {
        lindex_into(&argv[1], &argv[2].as_list()?)
    }
}

pub fn lindex_into(list: &Value, indices: &[Value]) -> MoltResult {
    let mut value: Value = list.clone();

    for index_val in indices {
        let list = value.as_list()?;
        let index = parse_list_index(index_val.as_str(), list.len() as MoltInt - 1)?;

        value = if index < 0 || index as usize >= list.len() {
            Value::empty()
        } else {
            list[index as usize].clone()
        };
    }

    molt_ok!(value)
}

/// # linsert list index ?element ...?
#[cfg(feature = "full")]
pub fn cmd_linsert<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 3, 0, "list index ?element ...?")?;
    let list = argv[1].as_list()?;
    let raw = parse_list_index(argv[2].as_str(), list.len() as MoltInt)?;
    let index = raw.clamp(0, list.len() as MoltInt) as usize;
    let mut output = Vec::with_capacity(list.len() + argv.len().saturating_sub(3));
    output.extend(list[..index].iter().cloned());
    output.extend(argv[3..].iter().cloned());
    output.extend(list[index..].iter().cloned());
    molt_ok!(output)
}

/// # list ?*arg*...?
///
/// Converts its arguments into a canonical list.
pub fn cmd_list<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    // No arg check needed; can take any number.
    molt_ok!(&argv[1..])
}

/// # llength *list*
///
/// Returns the length of the list.
pub fn cmd_llength<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 2, "list")?;

    molt_ok!(argv[1].as_list()?.len() as MoltInt)
}

/// # lrange list first last
#[cfg(feature = "full")]
pub fn cmd_lrange<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 4, 4, "list first last")?;
    let list = argv[1].as_list()?;
    let end = list.len() as MoltInt - 1;
    let first = parse_list_index(argv[2].as_str(), end)?.max(0) as usize;
    let last = parse_list_index(argv[3].as_str(), end)?.min(end);
    if first >= list.len() || last < first as MoltInt {
        return molt_ok!();
    }
    molt_ok!(Value::from(&list[first..=last as usize]))
}

/// # lrepeat positiveCount value ?value ...?
#[cfg(feature = "full")]
pub fn cmd_lrepeat<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 3, 0, "positiveCount value ?value ...?")?;
    let count = argv[1].as_int()?;
    if count < 1 {
        return molt_err!("must have a count of at least 1");
    }
    let count = usize::try_from(count)
        .map_err(|_| Exception::molt_err("list size overflow".into()))?;
    let capacity = count
        .checked_mul(argv.len() - 2)
        .ok_or_else(|| Exception::molt_err("list size overflow".into()))?;
    let mut output = Vec::with_capacity(capacity);
    for _ in 0..count {
        output.extend(argv[2..].iter().cloned());
    }
    molt_ok!(output)
}

/// # lreplace list first last ?element ...?
#[cfg(feature = "full")]
pub fn cmd_lreplace<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 4, 0, "list first last ?element ...?")?;
    let list = argv[1].as_list()?;
    let end = list.len() as MoltInt - 1;
    let raw_first = parse_list_index(argv[2].as_str(), end)?;
    if !list.is_empty() && raw_first >= list.len() as MoltInt {
        return molt_err!("list doesn't contain element {}", argv[2]);
    }
    let first = raw_first.max(0) as usize;
    let last = parse_list_index(argv[3].as_str(), end)?.min(end);
    let remove_end = if last < first as MoltInt { first } else { last as usize + 1 };
    let mut output = Vec::with_capacity(
        list.len().saturating_sub(remove_end.saturating_sub(first)) + argv.len() - 4,
    );
    output.extend(list[..usize::min(first, list.len())].iter().cloned());
    output.extend(argv[4..].iter().cloned());
    if remove_end < list.len() {
        output.extend(list[remove_end..].iter().cloned());
    }
    molt_ok!(output)
}

/// # lreverse list
#[cfg(feature = "full")]
pub fn cmd_lreverse<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 2, "list")?;
    let mut list = argv[1].to_list()?;
    list.reverse();
    molt_ok!(list)
}

fn parse_list_index(source: &str, end: MoltInt) -> Result<MoltInt, Exception> {
    let source = source.trim();
    if let Ok(index) = Value::get_int(source) {
        return Ok(index);
    }

    let (base, suffix) = if let Some(suffix) = source.strip_prefix("end") {
        (end, suffix)
    } else {
        let operator = source
            .char_indices()
            .skip(1)
            .find_map(|(index, ch)| matches!(ch, '+' | '-').then_some(index));
        let Some(operator) = operator else {
            return bad_list_index(source);
        };
        let base = Value::get_int(&source[..operator]).map_err(|_| {
            Exception::molt_err(
                format!(
                "bad index \"{source}\": must be integer?[+-]integer? or end?[+-]integer?"
            )
                .into(),
            )
        })?;
        (base, &source[operator..])
    };

    if suffix.is_empty() {
        return Ok(base);
    }
    let (operator, operand) = suffix.split_at(1);
    if !matches!(operator, "+" | "-") || operand.is_empty() {
        return bad_list_index(source);
    }
    let operand = Value::get_int(operand).map_err(|_| {
        Exception::molt_err(
            format!(
            "bad index \"{source}\": must be integer?[+-]integer? or end?[+-]integer?"
        )
            .into(),
        )
    })?;
    let index = match operator {
        "+" => base.checked_add(operand),
        "-" => base.checked_sub(operand),
        _ => unreachable!(),
    };
    index.ok_or_else(|| Exception::molt_err("integer overflow".into()))
}

fn bad_list_index(source: &str) -> Result<MoltInt, Exception> {
    molt_err!(
        "bad index \"{}\": must be integer?[+-]integer? or end?[+-]integer?",
        source
    )
}

/// # proc *name* *args* *body*
///
/// Defines a procedure.
pub fn cmd_proc<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 4, 4, "name args body")?;

    // FIRST, get the arguments
    let name = argv[1].as_str();
    let args = &*argv[2].as_list()?;

    validate_parameters(args)?;

    // NEXT, add the command.
    interp.add_proc(name, args, &argv[3]);

    molt_ok!()
}

/// # puts *string*
///
/// Outputs the string to stdout.
///
/// ## TCL Liens
///
/// * Does not support `-nonewline`
/// * Does not support `channelId`
pub fn cmd_puts<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 2, "string")?;
    #[cfg(feature = "std_buff")]
    {
        interp.std_buff.push(Ok(argv[1].clone()));
    }
    #[cfg(not(feature = "std_buff"))]
    {
        let _ = interp;
        println!("{}", argv[1]);
    }
    molt_ok!()
}

/// # rename *oldName* *newName*
///
/// Renames the procedure called *oldName* to *newName*. If *newName* is empty, the
/// procedure is removed.
pub fn cmd_rename<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 3, 3, "oldName newName")?;

    // FIRST, get the arguments
    let old_name = argv[1].as_str();
    let new_name = argv[2].as_str();

    if !interp.has_proc(old_name) {
        return molt_err!("can't rename \"{}\": command doesn't exist", old_name);
    }

    // NEXT, rename or remove the command.
    if new_name.is_empty() {
        interp.remove_proc(old_name);
    } else {
        interp.rename_proc(old_name, new_name);
    }

    molt_ok!()
}

/// # return ?-code code? ?-level level? ?value?
///
/// Returns from a proc with the given *value*, which defaults to the empty result.
/// See the documentation for **return** in The Molt Book for the option semantics.
///
/// ## TCL Liens
///
/// * Doesn't support all of TCL's fancy return machinery. Someday it will.
pub fn cmd_return<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 1, 0, "?options...? ?value?")?;

    // FIRST, set the defaults
    let mut code = ResultCode::Okay;
    let mut level: MoltInt = 1;
    let mut error_code: Option<Value> = None;
    let mut error_info: Option<Value> = None;

    // NEXT, with no arguments just return.
    if argv.len() == 1 {
        return Err(Exception::molt_return_ext(Value::empty(), level as usize, code));
    }

    // NEXT, get the return value: the last argument, if there's an odd number of arguments
    // after the command name.
    let return_value: Value;

    let opt_args: &[Value] = if argv.len().is_multiple_of(2) {
        // odd number of args following the command name
        return_value = argv[argv.len() - 1].clone();
        &argv[1..argv.len() - 1]
    } else {
        // even number of args following the command name
        return_value = Value::empty();
        &argv[1..argv.len()]
    };

    // NEXT, Get any options
    let mut queue = opt_args.iter();

    while let Some(opt) = queue.next() {
        // We built the queue to have an even number of arguments, and every option requires
        // a value; so there can't be a missing option value.
        let val = queue
            .next()
            .expect("missing option value: coding error in cmd_return");

        match opt.as_str() {
            "-code" => {
                code = ResultCode::from_value(val)?;
            }
            "-errorcode" => {
                error_code = Some(val.clone());
            }
            "-errorinfo" => {
                error_info = Some(val.clone());
            }
            "-level" => {
                // TODO: return better error:
                // bad -level value: expected non-negative integer but got "{}"
                level = val.as_int()?;
            }
            // TODO: In standard TCL there are no invalid options; all options are retained.
            _ => return molt_err!("invalid return option: \"{}\"", opt),
        }
    }

    // NEXT, return the result: normally a Return exception, but could be "Ok".
    if code == ResultCode::Error {
        Err(Exception::molt_return_err(
            return_value,
            level as usize,
            error_code,
            error_info,
        ))
    } else if level == 0 && code == ResultCode::Okay {
        // Not an exception!j
        Ok(return_value)
    } else {
        Err(Exception::molt_return_ext(return_value, level as usize, code))
    }
}

/// # set *varName* ?*newValue*?
///
/// Sets variable *varName* to *newValue*, returning the value.
/// If *newValue* is omitted, returns the variable's current value,
/// returning an error if the variable is unknown.
pub fn cmd_set<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 3, "varName ?newValue?")?;

    if argv.len() == 3 {
        interp.set_var_return(&argv[1], argv[2].clone())
    } else {
        molt_ok!(interp.var(&argv[1])?)
    }
}

/// # split string ?splitChars?
#[cfg(feature = "full")]
pub fn cmd_split<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 3, "string ?splitChars?")?;
    let source = argv[1].as_str();
    if source.is_empty() {
        return molt_ok!();
    }
    let split_chars = argv.get(2).map_or(" \n\t\r", Value::as_str);
    if split_chars.is_empty() {
        return molt_ok!(source
            .chars()
            .map(|ch| Value::from(ch.to_string()))
            .collect::<MoltList>());
    }
    molt_ok!(source
        .split(|ch| split_chars.contains(ch))
        .map(Value::from)
        .collect::<MoltList>())
}

/// # source *filename*
///
/// Sources the file, returning the result.
pub fn cmd_source<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 2, "filename")?;

    let filename = argv[1].as_str();

    match fs::read_to_string(filename) {
        Ok(script) => interp.eval(&script),
        Err(e) => molt_err!("couldn't read file \"{}\": {}", filename, e),
    }
}

/// # string *subcommand* ?*arg*...?
///
/// <https://www.tcl.tk/man/tcl8.6/TclCmd/string.htm>
pub fn cmd_string<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    let f = gen_subcommand!(
        Ctx,
        1,
        [
            ("cat", cmd_string_cat, "string cat ?string ...?"),
            (
                "compare",
                cmd_string_compare,
                "string compare ?-nocase? ?-length length? string1 string2"
            ),
            ("bytelength", cmd_string_bytelength, "string bytelength string"),
            (
                "equal",
                cmd_string_equal,
                "string equal ?-nocase? ?-length length? string1 string2"
            ),
            (
                "first",
                cmd_string_first,
                "string first needleString haystackString ?startIndex?"
            ),
            (
                "last",
                cmd_string_last,
                "string last needleString haystackString ?lastIndex?"
            ),
            ("index", cmd_string_index, "string index string charIndex"),
            (
                "is",
                cmd_string_is,
                "string is class ?-strict? ?-failindex varname? string"
            ),
            ("length", cmd_string_length, "string length string"),
            ("map", cmd_string_map, "string map ?-nocase? mapping string"),
            ("match", cmd_string_match, "string match ?-nocase? pattern string"),
            ("range", cmd_string_range, "string range string first last"),
            ("repeat", cmd_string_repeat, "string repeat string count"),
            (
                "replace",
                cmd_string_replace,
                "string replace string first last ?newstring?"
            ),
            ("reverse", cmd_string_reverse, "string reverse string"),
            ("tolower", cmd_string_tolower, "string tolower string ?first? ?last?"),
            ("toupper", cmd_string_toupper, "string toupper string ?first? ?last?"),
            ("trim", cmd_string_trim, "string trim string ?chars?"),
            ("trimleft", cmd_string_trim, "string trimleft string ?chars?"),
            ("trimright", cmd_string_trim, "string trimright string ?chars?"),
        ],
    );
    f(interp, argv)
}

/// # subst ?-nobackslashes? ?-nocommands? ?-novariables? string
#[cfg(feature = "full")]
pub fn cmd_subst<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 0, "?-nobackslashes? ?-nocommands? ?-novariables? string")?;
    let mut backslashes = true;
    let mut commands = true;
    let mut variables = true;
    for option in &argv[1..argv.len() - 1] {
        match option.as_str() {
            "-nobackslashes" => backslashes = false,
            "-nocommands" => commands = false,
            "-novariables" => variables = false,
            _ => {
                return molt_err!(
                    "bad option \"{}\": must be -nobackslashes, -nocommands, or -novariables",
                    option
                );
            }
        }
    }
    substitute(
        interp,
        argv.last().expect("argument count checked").as_str(),
        backslashes,
        commands,
        variables,
    )
}

#[cfg(feature = "full")]
fn substitute<Ctx>(
    interp: &mut Interp<Ctx>,
    source: &str,
    backslashes: bool,
    commands: bool,
    variables: bool,
) -> MoltResult {
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    while index < source.len() {
        let remaining = &source[index..];
        let ch = remaining.chars().next().expect("index is in bounds");
        if ch == '\\' && backslashes {
            let mut tokenizer = Tokenizer::new(remaining);
            output.push(tokenizer.backslash_subst());
            index = source.len() - tokenizer.as_str().len();
        } else if ch == '$' && variables {
            let mut context = EvalPtr::new(remaining);
            context.skip_char('$');
            if !context.next_is_varname_char() && !context.next_is('{') {
                output.push('$');
                index += 1;
                continue;
            }
            let word = parser::parse_varname(&mut context)?;
            output.push_str(interp.eval_word(&word)?.as_str());
            index = source.len() - context.tok().as_str().len();
        } else if ch == '[' && commands {
            let mut context = EvalPtr::new(remaining);
            context.skip_char('[');
            context.set_bracket_term(true);
            let script = parser::parse_script(&mut context)?;
            if !context.next_is(']') {
                return molt_err!("missing close-bracket");
            }
            context.next();
            index = source.len() - context.tok().as_str().len();
            match interp.eval_script(&script) {
                Ok(value) => output.push_str(value.as_str()),
                Err(exception) => match exception.code() {
                    ResultCode::Break => return molt_ok!(output),
                    ResultCode::Continue => {}
                    ResultCode::Return => output.push_str(exception.value().as_str()),
                    _ => return Err(exception),
                },
            }
        } else {
            output.push(ch);
            index += ch.len_utf8();
        }
    }
    molt_ok!(output)
}

#[cfg(feature = "full")]
#[derive(Clone, Copy)]
enum SwitchMode {
    Exact,
    Glob,
}

/// # switch ?switches? string pattern body ... ?default body?
#[cfg(feature = "full")]
pub fn cmd_switch<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 3, 0, "?switches? string pattern body ... ?default body?")?;
    let mut mode = SwitchMode::Exact;
    let mut nocase = false;
    let mut index = 1;
    while index < argv.len() && argv[index].as_str().starts_with('-') {
        match argv[index].as_str() {
            "-exact" => mode = SwitchMode::Exact,
            "-glob" => mode = SwitchMode::Glob,
            "-nocase" => nocase = true,
            "-regexp" | "-indexvar" | "-matchvar" => {
                return molt_err!("regular expression matching is not available");
            }
            "--" => {
                index += 1;
                break;
            }
            option => {
                return molt_err!(
                    "bad option \"{}\": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --",
                    option
                );
            }
        }
        index += 1;
    }
    if index >= argv.len() {
        return molt_err!(
            "wrong # args: should be \"switch ?switches? string pattern body ... ?default body?\""
        );
    }
    let source = argv[index].as_str();
    index += 1;
    let owned;
    let pairs = if argv.len() - index == 1 {
        owned = argv[index].as_list()?;
        owned.as_slice()
    } else {
        &argv[index..]
    };
    if !pairs.len().is_multiple_of(2) {
        return molt_err!("extra switch pattern with no body");
    }

    let mut selected = None;
    let mut default = None;
    for pair in (0..pairs.len()).step_by(2) {
        let pattern = pairs[pair].as_str();
        if pattern == "default" && pair + 2 == pairs.len() {
            default = Some(pair);
            continue;
        }
        let matches = match mode {
            SwitchMode::Exact => {
                if nocase {
                    pattern.to_lowercase() == source.to_lowercase()
                } else {
                    pattern == source
                }
            }
            SwitchMode::Glob => util::glob_match(pattern, source, nocase),
        };
        if matches {
            selected = Some(pair);
            break;
        }
    }
    let Some(mut selected) = selected.or(default) else {
        return molt_ok!();
    };
    while pairs[selected + 1].as_str() == "-" {
        selected += 2;
        if selected >= pairs.len() {
            return molt_err!(
                "no body specified for pattern \"{}\"",
                pairs[selected - 2]
            );
        }
    }
    interp.eval_value(&pairs[selected + 1])
}

/// string cat ?*arg* ...?
pub fn cmd_string_cat<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    let mut buff = String::new();

    for arg in &argv[2..] {
        buff.push_str(arg.as_str());
    }

    molt_ok!(buff)
}

/// string bytelength *string*
pub fn cmd_string_bytelength<Ctx>(
    _interp: &mut Interp<Ctx>,
    argv: &[Value],
) -> MoltResult {
    check_args(2, argv, 3, 3, "string")?;
    molt_ok!(argv[2].as_bytes().len() as MoltInt)
}

/// string compare ?-nocase? ?-length length? string1 string2
pub fn cmd_string_compare<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 7, "?-nocase? ?-length length? string1 string2")?;

    // FIRST, set the defaults.
    let arglen = argv.len();
    let mut nocase = false;
    let mut length: Option<MoltInt> = None;

    // NEXT, get options
    let opt_args = &argv[2..arglen - 2];
    let mut queue = opt_args.iter();

    while let Some(opt) = queue.next() {
        match opt.as_str() {
            "-nocase" => nocase = true,
            "-length" => {
                if let Some(val) = queue.next() {
                    length = Some(val.as_int()?);
                } else {
                    return molt_err!("wrong # args: should be \"string compare ?-nocase? ?-length length? string1 string2\"");
                }
            }
            _ => return molt_err!("bad option \"{}\": must be -nocase or -length", opt),
        }
    }

    if nocase {
        let val1 = &argv[arglen - 2];
        let val2 = &argv[arglen - 1];

        let val1 = val1.as_str().to_lowercase();
        let val2 = val2.as_str().to_lowercase();

        molt_ok!(util::compare_len(&val1, &val2, length)?)
    } else {
        molt_ok!(util::compare_len(
            argv[arglen - 2].as_str(),
            argv[arglen - 1].as_str(),
            length
        )?)
    }
}

/// string equal ?-nocase? ?-length length? string1 string2
pub fn cmd_string_equal<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 7, "?-nocase? ?-length length? string1 string2")?;

    // FIRST, set the defaults.
    let arglen = argv.len();
    let mut nocase = false;
    let mut length: Option<MoltInt> = None;

    // NEXT, get options
    let opt_args = &argv[2..arglen - 2];
    let mut queue = opt_args.iter();

    while let Some(opt) = queue.next() {
        match opt.as_str() {
            "-nocase" => nocase = true,
            "-length" => {
                if let Some(val) = queue.next() {
                    length = Some(val.as_int()?);
                } else {
                    return molt_err!("wrong # args: should be \"string equal ?-nocase? ?-length length? string1 string2\"");
                }
            }
            _ => return molt_err!("bad option \"{}\": must be -nocase or -length", opt),
        }
    }

    if nocase {
        let val1 = &argv[arglen - 2];
        let val2 = &argv[arglen - 1];

        let val1 = val1.as_str().to_lowercase();
        let val2 = val2.as_str().to_lowercase();

        let flag = util::compare_len(&val1, &val2, length)? == 0;
        molt_ok!(flag)
    } else {
        let flag = util::compare_len(
            argv[arglen - 2].as_str(),
            argv[arglen - 1].as_str(),
            length,
        )? == 0;
        molt_ok!(flag)
    }
}

/// string first *needleString* *haystackString* ?*startIndex*?
pub fn cmd_string_first<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 5, "needleString haystackString ?startIndex?")?;

    let needle = argv[2].as_str();
    let haystack = argv[3].as_str();

    let start_char: usize = if argv.len() == 5 {
        let arg = argv[4].as_int()?;

        if arg < 0 {
            0
        } else {
            arg as usize
        }
    } else {
        0
    };

    let pos_byte: Option<usize> = haystack
        .char_indices()
        .nth(start_char)
        .and_then(|(start_byte, _)| haystack[start_byte..].find(needle));

    let pos_char: MoltInt = match pos_byte {
        None => -1,
        Some(b) => {
            haystack[b..].char_indices().take_while(|(i, _)| *i < b).count() as MoltInt
                + start_char as MoltInt
        }
    };

    molt_ok!(pos_char)
}

/// string last *needleString* *haystackString* ?*lastIndex*?
pub fn cmd_string_last<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 5, "needleString haystackString ?lastIndex?")?;

    let needle = argv[2].as_str();
    let haystack = argv[3].as_str();

    let count = haystack.chars().count();

    let last: Option<usize> = if argv.len() == 5 {
        let arg = argv[4].as_int()?;

        if arg < 0 {
            return molt_ok!(-1);
        }

        if arg as usize >= count {
            None
        } else {
            Some(arg as usize)
        }
    } else {
        None
    };

    let slice = match last {
        None => haystack,
        Some(n) => match haystack.char_indices().nth(n + 1) {
            None => haystack,
            Some((byte, _)) => &haystack[..byte],
        },
    };

    let pos_byte = slice.rfind(needle);

    let pos_char: MoltInt = match pos_byte {
        None => -1,
        Some(b) => haystack.char_indices().take_while(|(i, _)| *i < b).count() as MoltInt,
    };

    molt_ok!(pos_char)
}

/// string index *string* *charIndex*
pub fn cmd_string_index<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 4, "string charIndex")?;
    let source = argv[2].as_str();
    let end = source.chars().count() as MoltInt - 1;
    let index = parse_list_index(argv[3].as_str(), end)?;
    if index < 0 {
        return molt_ok!();
    }
    molt_ok!(source
        .chars()
        .nth(index as usize)
        .map_or_else(String::new, |ch| ch.to_string()))
}

/// string is *class* ?-strict? ?-failindex *varname*? *string*
pub fn cmd_string_is<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 7, "class ?-strict? ?-failindex varname? string")?;
    let class = argv[2].as_str();
    let valid_class = matches!(
        class,
        "alnum"
            | "alpha"
            | "ascii"
            | "boolean"
            | "control"
            | "digit"
            | "double"
            | "false"
            | "graph"
            | "integer"
            | "list"
            | "lower"
            | "print"
            | "punct"
            | "space"
            | "true"
            | "upper"
            | "wideinteger"
            | "wordchar"
            | "xdigit"
    );
    if !valid_class {
        return molt_err!(
            "bad class \"{}\": must be alnum, alpha, ascii, control, boolean, digit, double, false, graph, integer, list, lower, print, punct, space, true, upper, wideinteger, wordchar, or xdigit",
            class
        );
    }

    let mut strict = false;
    let mut failindex = None;
    let mut index = 3;
    while index + 1 < argv.len() {
        match argv[index].as_str() {
            "-strict" => strict = true,
            "-failindex" if index + 2 < argv.len() => {
                failindex = Some(&argv[index + 1]);
                index += 1;
            }
            option => {
                return molt_err!(
                    "bad option \"{}\": must be -failindex or -strict",
                    option
                );
            }
        }
        index += 1;
    }
    if index + 1 != argv.len() {
        return molt_err!(
            "wrong # args: should be \"string is class ?-strict? ?-failindex varname? string\""
        );
    }
    let source = argv[index].as_str();
    let (valid, failure) =
        if source.is_empty() { (!strict, 0) } else { string_classify(class, source) };
    if let Some(variable) = failindex {
        let offset = if valid { source.chars().count() } else { failure };
        interp.set_var(variable, Value::from(offset as MoltInt))?;
    }
    molt_ok!(valid)
}

fn string_classify(class: &str, source: &str) -> (bool, usize) {
    let whole = match class {
        "boolean" => Some(Value::get_bool(source).is_ok()),
        "true" => Some(Value::get_bool(source) == Ok(true)),
        "false" => Some(Value::get_bool(source) == Ok(false)),
        "double" => Some(Value::get_float(source).is_ok() || string_is_integer(source)),
        "integer" | "wideinteger" => Some(string_is_integer(source)),
        "list" => Some(Value::from(source).as_list().is_ok()),
        _ => None,
    };
    if let Some(valid) = whole {
        return (
            valid,
            if valid { source.chars().count() } else { numeric_failure(source) },
        );
    }

    for (index, ch) in source.chars().enumerate() {
        let valid = match class {
            "alnum" => ch.is_alphanumeric(),
            "alpha" => ch.is_alphabetic(),
            "ascii" => ch.is_ascii(),
            "control" => ch.is_control(),
            "digit" => ch.is_numeric(),
            "graph" => !ch.is_control() && !ch.is_whitespace(),
            "lower" => ch.is_lowercase(),
            "print" => !ch.is_control(),
            "punct" => ch.is_ascii_punctuation(),
            "space" => ch.is_whitespace(),
            "upper" => ch.is_uppercase(),
            "wordchar" => ch.is_alphanumeric() || ch == '_',
            "xdigit" => ch.is_ascii_hexdigit(),
            _ => unreachable!("class checked by caller"),
        };
        if !valid {
            return (false, index);
        }
    }
    (true, source.chars().count())
}

fn string_is_integer(source: &str) -> bool {
    #[cfg(feature = "full")]
    {
        Value::get_bignum(source).is_ok()
    }
    #[cfg(not(feature = "full"))]
    {
        Value::get_int(source).is_ok()
    }
}

fn numeric_failure(source: &str) -> usize {
    let mut seen_digit = false;
    for (index, ch) in source.chars().enumerate() {
        let accepted = ch.is_ascii_digit()
            || (index == 0 && matches!(ch, '+' | '-'))
            || (index == 1 && ch == 'x' && source.starts_with('0'));
        if !accepted {
            return index;
        }
        seen_digit |= ch.is_ascii_digit();
    }
    if seen_digit {
        source.chars().count()
    } else {
        0
    }
}

/// string length *string*
pub fn cmd_string_length<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 3, "string")?;

    let len: MoltInt = argv[2].as_str().chars().count() as MoltInt;
    molt_ok!(len)
}

/// string map ?-nocase? *charMap* *string*
pub fn cmd_string_map<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 5, "?-nocase? charMap string")?;

    let mut nocase = false;

    if argv.len() == 5 {
        let opt = argv[2].as_str();

        if opt == "-nocase" {
            nocase = true;
        } else {
            return molt_err!("bad option \"{}\": must be -nocase", opt);
        }
    }

    let char_map = argv[argv.len() - 2].as_dict()?;
    let s = argv[argv.len() - 1].as_str();

    let filtered_keys: Vec<(Cow<'_, str>, usize, &Value)> = char_map
        .iter()
        .map(|(k, v)| {
            let key = if nocase {
                Cow::Owned(k.as_str().to_lowercase())
            } else {
                Cow::Borrowed(k.as_str())
            };

            let count = key.chars().count();

            (key, count, v)
        })
        .filter(|(_, count, _)| *count > 0)
        .collect::<Vec<_>>();

    let string_lower: Option<String> = if nocase { Some(s.to_lowercase()) } else { None };

    let mut result = String::new();
    let mut skip = 0;

    for (i, c) in s.char_indices() {
        if skip > 0 {
            skip -= 1;
            continue;
        }

        let mut matched = false;

        for (from, from_char_count, to) in &filtered_keys {
            let haystack: &str = match &string_lower {
                Some(x) => &x[i..],
                None => &s[i..],
            };

            if haystack.starts_with(from.as_ref()) {
                matched = true;

                result.push_str(to.as_str());
                skip = from_char_count - 1;

                break;
            }
        }

        if !matched {
            result.push(c);
        }
    }

    molt_ok!(result)
}

/// string match ?-nocase? *pattern* *string*
pub fn cmd_string_match<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 5, "?-nocase? pattern string")?;
    let nocase = argv.len() == 5;
    if nocase && argv[2].as_str() != "-nocase" {
        return molt_err!("bad option \"{}\": must be -nocase", argv[2]);
    }
    let pattern = argv[argv.len() - 2].as_str();
    let source = argv[argv.len() - 1].as_str();
    molt_ok!(util::glob_match(pattern, source, nocase))
}

/// string range *string* *first* *last*
pub fn cmd_string_range<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 5, 5, "string first last")?;

    let s = argv[2].as_str();
    let first = argv[3].as_int()?;
    let last = argv[4].as_int()?;

    if last < 0 {
        return molt_ok!("");
    }

    let clamp = { |i: MoltInt| if i < 0 { 0 } else { i } };

    let substr = s
        .chars()
        .skip(clamp(first) as usize)
        .take((clamp(last) - clamp(first) + 1) as usize)
        .collect::<String>();

    molt_ok!(substr)
}

/// string repeat *string* *count*
pub fn cmd_string_repeat<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 4, 4, "string count")?;
    let count = argv[3].as_int()?.max(0) as usize;
    molt_ok!(argv[2].as_str().repeat(count))
}

/// string replace *string* *first* *last* ?*newstring*?
pub fn cmd_string_replace<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 5, 6, "string first last ?newstring?")?;
    let source = argv[2].as_str();
    let chars: Vec<char> = source.chars().collect();
    let end = chars.len() as MoltInt - 1;
    let first = parse_list_index(argv[3].as_str(), end)?.max(0) as usize;
    let last = parse_list_index(argv[4].as_str(), end)?;
    if first >= chars.len() || last < first as MoltInt {
        return molt_ok!(source);
    }
    let after = usize::min(last.saturating_add(1) as usize, chars.len());
    let replacement = argv.get(5).map_or("", Value::as_str);
    let mut output = String::with_capacity(source.len() + replacement.len());
    output.extend(chars[..first].iter());
    output.push_str(replacement);
    output.extend(chars[after..].iter());
    molt_ok!(output)
}

/// string reverse *string*
pub fn cmd_string_reverse<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 3, "string")?;
    molt_ok!(argv[2].as_str().chars().rev().collect::<String>())
}

/// string tolower *string*
pub fn cmd_string_tolower<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    string_change_case(argv, false)
}

/// string toupper *string*
pub fn cmd_string_toupper<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    string_change_case(argv, true)
}

fn string_change_case(argv: &[Value], uppercase: bool) -> MoltResult {
    check_args(2, argv, 3, 5, "string ?first? ?last?")?;
    let source = argv[2].as_str();
    if argv.len() == 3 {
        return molt_ok!(if uppercase {
            source.to_uppercase()
        } else {
            source.to_lowercase()
        });
    }
    let chars: Vec<char> = source.chars().collect();
    let end = chars.len() as MoltInt - 1;
    let first = parse_list_index(argv[3].as_str(), end)?.max(0) as usize;
    let last = if argv.len() == 5 {
        parse_list_index(argv[4].as_str(), end)?.min(end)
    } else {
        first as MoltInt
    };
    if first >= chars.len() || last < first as MoltInt {
        return molt_ok!(source);
    }
    let after = last.saturating_add(1) as usize;
    let middle: String = chars[first..after].iter().collect();
    let changed = if uppercase { middle.to_uppercase() } else { middle.to_lowercase() };
    let mut output = String::with_capacity(source.len());
    output.extend(chars[..first].iter());
    output.push_str(&changed);
    output.extend(chars[after..].iter());
    molt_ok!(output)
}

/// string (trim|trimleft|trimright) *string*
pub fn cmd_string_trim<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(2, argv, 3, 4, "string ?chars?")?;

    let s = argv[2].as_str();
    let chars = argv.get(3).map(Value::as_str);
    let matches =
        |ch: char| chars.map_or_else(|| ch.is_whitespace(), |set| set.contains(ch));
    let trimmed = match argv[1].as_str() {
        "trimleft" => s.trim_start_matches(matches),
        "trimright" => s.trim_end_matches(matches),
        _ => s.trim_matches(matches),
    };

    molt_ok!(trimmed)
}

/// throw *type* *message*
///
/// Throws an error with the error code and message.
pub fn cmd_throw<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 3, 3, "type message")?;

    Err(Exception::molt_err2(argv[1].clone(), argv[2].clone()))
}

/// # time *command* ?*count*?
///
/// Executes the command the given number of times, and returns the average
/// number of microseconds per iteration.  The *count* defaults to 1.
pub fn cmd_time<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 3, "command ?count?")?;

    let command = &argv[1];

    let count = if argv.len() == 3 { argv[2].as_int()? } else { 1 };

    let start = Instant::now();

    for _i in 0..count {
        interp.eval_value(command)?;
    }

    let span = start.elapsed();

    let avg = if count > 0 { span.as_nanos() / (count as u128) } else { 0 } as MoltInt;

    molt_ok!("{} nanoseconds per iteration", avg)
}

#[cfg(feature = "full")]
#[derive(Clone, Copy)]
enum TryHandlerKind {
    On(ResultCode),
    Trap,
}

#[cfg(feature = "full")]
#[derive(Clone, Copy)]
struct TryHandler {
    kind: TryHandlerKind,
    match_index: usize,
    variables_index: usize,
    body_index: usize,
}

/// # try body ?handler ...? ?finally script?
#[cfg(feature = "full")]
pub fn cmd_try<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 0, "body ?handler ...? ?finally script?")?;
    let mut handlers = Vec::new();
    let mut finally = None;
    let mut index = 2;
    while index < argv.len() {
        match argv[index].as_str() {
            "finally" => {
                if index + 2 != argv.len() {
                    return molt_err!("wrong # args: finally clause must be last");
                }
                finally = Some(index + 1);
                break;
            }
            "on" | "trap" if index + 3 < argv.len() => {
                let variables = argv[index + 2].as_list()?;
                if variables.len() > 2 {
                    return molt_err!(
                        "handler variable list must have at most two elements"
                    );
                }
                let kind = if argv[index].as_str() == "on" {
                    let code =
                        argv[index + 1].as_str().parse::<ResultCode>().map_err(|_| {
                            Exception::molt_err(
                                format!("bad completion code \"{}\"", argv[index + 1])
                                    .into(),
                            )
                        })?;
                    TryHandlerKind::On(code)
                } else {
                    argv[index + 1].as_list()?;
                    TryHandlerKind::Trap
                };
                handlers.push(TryHandler {
                    kind,
                    match_index: index + 1,
                    variables_index: index + 2,
                    body_index: index + 3,
                });
                index += 4;
            }
            clause => {
                return molt_err!(
                    "bad handler type \"{}\": must be finally, on, or trap",
                    clause
                )
            }
        }
    }

    let mut result = interp.eval_value(&argv[1]);
    let code = match &result {
        Ok(_) => ResultCode::Okay,
        Err(exception) => exception.code(),
    };
    let selected = handlers.iter().find(|handler| match handler.kind {
        TryHandlerKind::On(expected) => expected == code,
        TryHandlerKind::Trap => {
            if let Err(exception) = &result {
                if exception.code() == ResultCode::Error {
                    let pattern = argv[handler.match_index]
                        .as_list()
                        .expect("trap pattern was validated");
                    let error_code = exception
                        .error_code()
                        .as_list()
                        .expect("error code is always a Tcl list");
                    return pattern.len() <= error_code.len()
                        && pattern.iter().zip(error_code.iter()).all(|(a, b)| a == b);
                }
            }
            false
        }
    });

    if let Some(handler) = selected {
        let variables = argv[handler.variables_index]
            .as_list()
            .expect("handler variables were validated");
        let value = match &result {
            Ok(value) => value.clone(),
            Err(exception) => exception.value(),
        };
        let options = interp.return_options(&result);
        if let Some(variable) = variables.first() {
            interp.set_var(variable, value)?;
        }
        if let Some(variable) = variables.get(1) {
            interp.set_var(variable, options)?;
        }
        result = interp.eval_value(&argv[handler.body_index]);
    }

    if let Some(finally) = finally {
        interp.eval_value(&argv[finally])?;
    }
    result
}

/// # unset ?-nocomplain? *varName*
///
/// Removes the variable from the interpreter.  This is a no op if
/// there is no such variable.  The -nocomplain option is accepted for
/// compatible with standard TCL, but is never required.
pub fn cmd_unset<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 1, 0, "?-nocomplain? ?--? ?name name name...?")?;

    let mut options_ok = true;

    for arg in argv {
        let var = arg.as_str();

        if options_ok {
            if var == "--" {
                options_ok = false;
                continue;
            } else if var == "-nocomplain" {
                continue;
            }
        }

        interp.unset_var(arg);
    }

    molt_ok!()
}

/// # uplevel ?level? arg ?arg ...?
#[cfg(feature = "full")]
pub fn cmd_uplevel<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 0, "?level? arg ?arg ...?")?;
    let (level, first_script) = if looks_like_level(argv[1].as_str()) {
        (scope_level(interp, argv[1].as_str())?, 2)
    } else {
        (scope_level(interp, "1")?, 1)
    };
    if first_script >= argv.len() {
        return molt_err!("wrong # args: should be \"uplevel ?level? arg ?arg ...?\"");
    }
    let script = concatenate_values(&argv[first_script..])?;
    interp.eval_at_scope(level, &script)
}

/// # upvar ?level? otherVar localVar ?otherVar localVar ...?
#[cfg(feature = "full")]
pub fn cmd_upvar<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 3, 0, "?level? otherVar localVar ?otherVar localVar ...?")?;
    let has_level = argv.len().is_multiple_of(2);
    let first_pair = usize::from(has_level) + 1;
    if !(argv.len() - first_pair).is_multiple_of(2) {
        return molt_err!(
            "wrong # args: should be \"upvar ?level? otherVar localVar ?otherVar localVar ...?\""
        );
    }
    let level = if has_level {
        scope_level(interp, argv[1].as_str())?
    } else {
        scope_level(interp, "1")?
    };
    for pair in argv[first_pair..].chunks_exact(2) {
        if level == interp.scope_level() && pair[0] == pair[1] {
            return molt_err!("can't upvar from variable to itself");
        }
        interp.upvar_as(level, pair[0].as_str(), pair[1].as_str());
    }
    molt_ok!()
}

#[cfg(feature = "full")]
fn looks_like_level(source: &str) -> bool {
    source.starts_with('#') || Value::get_int(source).is_ok()
}

#[cfg(feature = "full")]
fn scope_level<Ctx>(interp: &Interp<Ctx>, source: &str) -> Result<usize, Exception> {
    let current = interp.scope_level();
    let level = if let Some(absolute) = source.strip_prefix('#') {
        Value::get_int(absolute)?
    } else {
        let relative = Value::get_int(source)?;
        if relative < 0 {
            return molt_err!("bad level \"{}\"", source);
        }
        let relative = usize::try_from(relative)
            .map_err(|_| Exception::molt_err(format!("bad level \"{source}\"").into()))?;
        return current.checked_sub(relative).ok_or_else(|| {
            Exception::molt_err(format!("bad level \"{source}\"").into())
        });
    };
    if level < 0 || level as usize > current {
        molt_err!("bad level \"{}\"", source)
    } else {
        Ok(level as usize)
    }
}

#[cfg(feature = "full")]
fn concatenate_values(values: &[Value]) -> Result<Value, Exception> {
    if values.len() == 1 {
        return Ok(values[0].clone());
    }
    let mut words = Vec::new();
    for value in values {
        words.extend(value.as_list()?.iter().cloned());
    }
    Ok(Value::from(list_to_string(&words)))
}

/// # while *test* *command*
///
/// A standard "while" loop.  *test* is a boolean expression; *command* is a script to
/// execute so long as the expression is true.
pub fn cmd_while<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 3, 3, "test command")?;

    while interp.expr_bool(&argv[1])? {
        let result = interp.eval_value(&argv[2]);

        if let Err(exception) = result {
            match exception.code() {
                ResultCode::Break => break,
                ResultCode::Continue => (),
                _ => return Err(exception),
            }
        }
    }

    molt_ok!()
}
