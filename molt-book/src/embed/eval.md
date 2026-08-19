# Evaluating Molt Code

An application can evaluate Molt in several ways:

* `Interp::eval` evaluates a string containing a command or script.
* `Interp::eval_value` evaluates a script already stored in a `Value`, reusing its parsed form.
* `Interp::expr`, `expr_bool`, `expr_int`, and `expr_float` evaluate expression `Value`s.
* `molt_shell::repl` provides an interactive shell.
* `molt_shell::script` evaluates a script file with command-line arguments.

## Evaluating scripts

`Interp::eval` returns the last command's value or an `Exception`:

```rust
use molt_forked::{Interp, Value};

let mut interp = Interp::default();
let value: Value = interp.eval("set answer [expr {6 * 7}]")?;
assert_eq!(value.as_int()?, 42);
```

When a script is already a `Value`, prefer `eval_value`. Its parsed representation is cached,
which avoids reparsing loop bodies and other repeatedly evaluated scripts.

## Evaluating control-structure bodies

This simplified version of Molt's `while` command shows `eval_value` and result-code handling:

```rust
pub fn cmd_while<Ctx>(interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 3, 3, "test command")?;

    while interp.expr_bool(&argv[1])? {
        if let Err(exception) = interp.eval_value(&argv[2]) {
            match exception.code() {
                ResultCode::Break => break,
                ResultCode::Continue => (),
                _ => return Err(exception),
            }
        }
    }

    molt_ok!()
}
```

See [The `MoltResult` Type](./molt_result.md) for exception details.

## Evaluating expressions

Expression methods accept a `Value` so their parsed representation can also be reused:

```rust
use molt_forked::{Interp, Value};

let mut interp = Interp::default();
let expression = Value::from("1 + 1");
let value = interp.expr(&expression)?;
assert_eq!(value.as_int()?, 2);

let comparison = Value::from("2 < 3");
assert!(interp.expr_bool(&comparison)?);
```

## Providing an interactive REPL

```rust
use molt_forked::Interp;

let mut interp = Interp::default();
molt_shell::repl(&mut interp);
```

The REPL prompt can be set through the `tcl_prompt1` variable; see the
[molt shell](../cmdline/molt_shell.md) documentation.

## Evaluating script files

`molt_shell::script` sets the Molt `arg0` and `argv` variables before evaluating a file:

```rust
use molt_forked::Interp;
use std::env;

let args: Vec<String> = env::args().collect();
let mut interp = Interp::default();

if args.len() > 1 {
    molt_shell::script(&mut interp, &args[1..]);
} else {
    eprintln!("Usage: myshell filename.tcl");
}
```
