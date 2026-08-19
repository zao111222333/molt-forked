use molt_forked::prelude::*;
use molt_forked::syntax::{self, ParseStatus, SyntaxAnalysis, SyntaxKind};
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Editor, Helper};
use std::borrow::Cow;
use std::cell::RefCell;
use std::fs;

#[derive(Debug)]
struct CachedAnalysis {
    source: String,
    analysis: SyntaxAnalysis,
}

/// Rustyline adapter backed by Molt's lossless Tcl parser.
#[derive(Debug)]
struct MoltHelper {
    cache: RefCell<Option<CachedAnalysis>>,
    color: bool,
    #[cfg(test)]
    parse_count: std::cell::Cell<usize>,
}

impl MoltHelper {
    fn new() -> Self {
        Self {
            cache: RefCell::new(None),
            color: std::env::var_os("NO_COLOR").is_none(),
            #[cfg(test)]
            parse_count: std::cell::Cell::new(0),
        }
    }

    fn ensure_analysis(&self, source: &str) {
        let current = self.cache.borrow();
        if current.as_ref().is_some_and(|cached| cached.source == source) {
            return;
        }
        drop(current);

        let analysis = syntax::analyze_script(source);
        #[cfg(test)]
        self.parse_count.set(self.parse_count.get() + 1);
        *self.cache.borrow_mut() =
            Some(CachedAnalysis { source: source.to_owned(), analysis });
    }

    fn status(&self, source: &str) -> ParseStatus {
        self.ensure_analysis(source);
        self.cache
            .borrow()
            .as_ref()
            .expect("analysis was just populated")
            .analysis
            .status()
    }

    fn highlighted(&self, source: &str) -> String {
        self.ensure_analysis(source);
        let cache = self.cache.borrow();
        render_ansi(
            source,
            &cache.as_ref().expect("analysis was just populated").analysis,
        )
    }
}

impl Completer for MoltHelper {
    type Candidate = String;
}

impl Hinter for MoltHelper {
    type Hint = String;
}

impl Highlighter for MoltHelper {
    fn highlight<'line>(&self, line: &'line str, _pos: usize) -> Cow<'line, str> {
        if !self.color || line.is_empty() {
            return Cow::Borrowed(line);
        }
        Cow::Owned(self.highlighted(line))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, kind: CmdKind) -> bool {
        kind != CmdKind::ForcedRefresh
    }
}

impl Validator for MoltHelper {
    fn validate(
        &self,
        context: &mut ValidationContext<'_>,
    ) -> rustyline::Result<ValidationResult> {
        Ok(if self.status(context.input()).is_incomplete() {
            ValidationResult::Incomplete
        } else {
            // Invalid-but-complete Tcl is submitted so the interpreter can produce the real
            // structured Tcl error instead of trapping the user in the editor.
            ValidationResult::Valid(None)
        })
    }
}

impl Helper for MoltHelper {}

fn render_ansi(source: &str, analysis: &SyntaxAnalysis) -> String {
    let mut output = String::with_capacity(source.len() + analysis.tokens().len() * 8);
    let mut active = "";

    for token in analysis.tokens() {
        let style = ansi_style(token.kind(), token.depth());
        if style != active {
            output.push_str("\x1b[0m");
            output.push_str(style);
            active = style;
        }
        let range = token.range();
        output.push_str(&source[range.start()..range.end()]);
    }
    if !active.is_empty() {
        output.push_str("\x1b[0m");
    }
    output
}

fn ansi_style(kind: SyntaxKind, depth: u16) -> &'static str {
    match kind {
        SyntaxKind::Plain | SyntaxKind::Whitespace | SyntaxKind::Word => "",
        SyntaxKind::Comment => "\x1b[2;90m",
        SyntaxKind::Command => "\x1b[1;36m",
        SyntaxKind::String => "\x1b[32m",
        SyntaxKind::Variable => "\x1b[33m",
        SyntaxKind::Escape => "\x1b[35m",
        SyntaxKind::Delimiter => match depth % 4 {
            0 => "\x1b[36m",
            1 => "\x1b[35m",
            2 => "\x1b[34m",
            _ => "\x1b[33m",
        },
        SyntaxKind::Separator => "\x1b[90m",
        SyntaxKind::Number => "\x1b[34m",
        SyntaxKind::Operator => "\x1b[1;35m",
        SyntaxKind::Function => "\x1b[36m",
        SyntaxKind::Invalid => "\x1b[4;31m",
        _ => "",
    }
}

/// Invokes an interactive REPL for the given interpreter, using `rustyline` line editing.
///
/// The REPL will display a default prompt to the user.  Press `^C` to terminate
/// the REPL, returning control to the caller.  Entering `exit` will also normally cause the
/// application to terminate (but the `exit` command can be removed or redefined by the
/// application).
///
/// To change the prompt, set the `tcl_prompt1` TCL variable to a script that returns
/// the desired prompt.
///
/// See [`molt_forked::interp`] for details on how to configure a Molt interpreter.
///
/// # Example
///
/// ```
/// use molt_forked::Interp;
///
/// // FIRST, create and initialize the interpreter.
/// let mut interp = Interp::default();
///
/// // NEXT, invoke the REPL.
/// molt_shell::repl(&mut interp);
/// ```
pub fn repl<Ctx: 'static>(interp: &mut Interp<Ctx>) {
    let mut rl = match Editor::<MoltHelper, DefaultHistory>::new() {
        Ok(editor) => editor,
        Err(error) => {
            eprintln!("unable to initialize line editor: {error}");
            return;
        }
    };
    rl.set_helper(Some(MoltHelper::new()));

    loop {
        let readline = if let Ok(pscript) = interp.scalar("tcl_prompt1") {
            match interp.eval(pscript.as_str()) {
                Ok(prompt) => rl.readline(prompt.as_str()),
                Err(exception) => {
                    println!("{}", exception.value());
                    rl.readline("% ")
                }
            }
        } else {
            rl.readline("% ")
        };

        match readline {
            Ok(line) => {
                if !line.trim().is_empty() {
                    match interp.eval(&line) {
                        Ok(value) => {
                            let _ = rl.add_history_entry(line.as_str());

                            // Don't output empty values.
                            if !value.as_str().is_empty() {
                                println!("{}", value);
                            }
                        }
                        Err(exception) => {
                            let _ = rl.add_history_entry(line.as_str());
                            println!("{}", exception.value());
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                break;
            }
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("I/O Error: {:?}", err);
                break;
            }
        }
    }
}

/// Executes a script from a set of command line arguments.
///
/// `args[0]` is presumed to be the name of a Molt script file, with any subsequent
/// arguments being arguments to pass to the script.  The script will be be executed in
/// the context of the given interpreter.
///
/// # Molt Variables
///
/// The calling information will be passed to the interpreter in the form of Molt
/// variables:
///
/// * The Molt variable `arg0` will be set to the `arg0` value.
/// * The Molt variable `argv` will be set to a Molt list containing the remainder of the
///   `argv` array.
///
/// See [`molt_forked::interp`] for details on how to configure a Molt interpreter.
///
/// # Example
///
/// ```
/// use molt_forked::Interp;
/// use std::env;
///
/// // FIRST, get the command line arguments.
/// let args: Vec<String> = env::args().collect();
///
/// // NEXT, create and initialize the interpreter.
/// let mut interp = Interp::default();
///
/// // NEXT, evaluate the file, if any.
/// if args.len() > 1 {
///     molt_shell::script(&mut interp, &args[1..]);
/// } else {
///     eprintln!("Usage: myshell *filename.tcl");
/// }
/// ```
pub fn script<Ctx: 'static>(interp: &mut Interp<Ctx>, args: &[String]) {
    let arg0 = &args[0];
    let argv = &args[1..];
    match fs::read_to_string(&args[0]) {
        Ok(script) => execute_script(interp, script, arg0, argv),
        Err(e) => println!("{}", e),
    }
}

/// Executes a script read from a file, with any command-line arguments, in
/// the context of the given interpreter.  The `script` is the text of the
/// script, `arg0` is the name of the script file, and `argv` contains the script
/// arguments.
///
/// # Molt Variables
///
/// The calling information will be passed to the interpreter in the form of Molt
/// variables:
///
/// * The Molt variable `arg0` will be set to the `arg0` value.
/// * The Molt variable `argv` will be set to the `argv` array as a Molt list.
fn execute_script<Ctx: 'static>(
    interp: &mut Interp<Ctx>,
    script: String,
    arg0: &str,
    argv: &[String],
) {
    let argv: MoltList = argv.iter().map(Value::from).collect();
    interp
        .set_scalar("arg0", Value::from(arg0))
        .expect("arg0 predefined as array!");
    interp
        .set_scalar("argv", Value::from(argv))
        .expect("argv predefined as array!");

    match interp.eval(&script) {
        Ok(_) => (),
        Err(exception) => {
            eprintln!("{}", exception.value());
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod highlight_tests {
    use super::*;
    use std::fmt::Write as _;

    fn strip_ansi(source: &str) -> String {
        let mut output = String::new();
        let mut chars = source.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\x1b' && chars.peek() == Some(&'[') {
                chars.next();
                for code in chars.by_ref() {
                    if code == 'm' {
                        break;
                    }
                }
            } else {
                output.write_char(ch).unwrap();
            }
        }
        output
    }

    #[test]
    fn cache_is_shared_by_highlighter_and_validator_status() {
        let helper = MoltHelper::new();
        let line = "if {$value > 1} {puts ok}";
        assert_eq!(strip_ansi(&helper.highlighted(line)), line);
        assert_eq!(helper.status(line), ParseStatus::Complete);
        assert_eq!(helper.parse_count.get(), 1);
    }

    #[test]
    fn incomplete_and_invalid_have_distinct_submission_behavior() {
        let helper = MoltHelper::new();
        assert!(helper.status("set value {").is_incomplete());
        assert_eq!(helper.status("set value {x}tail"), ParseStatus::Invalid);
    }

    #[test]
    fn ansi_highlighting_preserves_text_and_unicode() {
        let helper = MoltHelper::new();
        let line = "# 注释\nputs \"值=$value\"";
        let highlighted = helper.highlighted(line);
        assert_eq!(strip_ansi(&highlighted), line);
        assert!(highlighted.contains("\x1b["));
    }
}
