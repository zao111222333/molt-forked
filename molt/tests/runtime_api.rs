use std::collections::BTreeMap;

use molt_forked::{
    compile, gen_command, molt_ok, CommandSet, EnvironmentPolicy, Interp, InterpConfig,
    MoltContext, MoltResult, Program, Value,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CaptureContext {
    calls: Vec<String>,
}

impl MoltContext for CaptureContext {
    fn command_set() -> CommandSet<Self> {
        gen_command!(CaptureContext, [], [("capture", capture, "capture value")])
    }
}

fn capture(interp: &mut Interp<CaptureContext>, argv: &[Value]) -> MoltResult {
    let value = argv.get(1).map(Value::as_str).unwrap_or_default();
    interp.context_mut().calls.push(value.to_owned());
    molt_ok!(value)
}

#[test]
fn context_oriented_lifecycle_returns_the_context() {
    let mut interp = Interp::new(CaptureContext::default());
    assert_eq!(interp.eval("capture first").unwrap().as_str(), "first");
    assert_eq!(interp.into_context(), CaptureContext { calls: vec!["first".to_owned()] });
}

#[test]
fn explicit_environment_is_deterministic() {
    let mut environment = BTreeMap::new();
    environment.insert("SVRF_TEST".to_owned(), "explicit".to_owned());
    let config = InterpConfig {
        environment: EnvironmentPolicy::Explicit(environment),
        ..InterpConfig::default()
    };
    let mut interp = Interp::with_config(CaptureContext::default(), config);

    assert_eq!(interp.eval("set env(SVRF_TEST)").unwrap().as_str(), "explicit");
}

#[test]
fn compiled_program_matches_direct_evaluation() {
    let program = compile("set value 40; incr value 2").unwrap();
    assert_eq!(program.source(), "set value 40; incr value 2");

    let mut compiled_interp = Interp::new(CaptureContext::default());
    let mut direct_interp = Interp::new(CaptureContext::default());
    assert_eq!(
        compiled_interp.eval_program(&program),
        direct_interp.eval(program.source())
    );

    let cloned = Program::compile(program.source()).unwrap();
    assert_eq!(program, cloned);
}

#[test]
fn compatibility_builder_still_works() {
    let command_set = CaptureContext::command_set();
    let mut interp =
        molt_forked::InterpBuilder::new(CaptureContext::default(), command_set)
            .environment(false)
            .name("compat")
            .build();
    assert_eq!(interp.eval("capture legacy").unwrap().as_str(), "legacy");
    assert_eq!(interp.name(), "compat");
}

#[derive(Debug, Default)]
struct StandardOnlyContext;

impl MoltContext for StandardOnlyContext {
    fn command_set() -> CommandSet<Self> {
        CommandSet::standard()
    }
}

#[test]
fn generic_context_can_use_only_standard_commands() {
    let mut interp = Interp::new(StandardOnlyContext);
    assert_eq!(interp.eval("expr {6 * 7}").unwrap(), Value::from(42));
    assert!(interp.eval("missing_application_command").is_err());
}

#[test]
fn cloned_interpreter_is_an_independent_transactional_snapshot() {
    let mut original = Interp::new(CaptureContext::default());
    original
        .eval("set value original; proc read_value {} {global value; return $value}")
        .unwrap();

    let mut staging = original.clone();
    staging.eval("set value staging; capture staged").unwrap();

    assert_eq!(staging.eval("read_value").unwrap().as_str(), "staging");
    assert_eq!(staging.context().calls, vec!["staged"]);
    assert_eq!(original.eval("read_value").unwrap().as_str(), "original");
    assert!(original.context().calls.is_empty());
}
