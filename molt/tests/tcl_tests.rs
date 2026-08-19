use molt_forked::prelude::*;
#[test]
fn test_tcl_tests() {
    // FIRST, create and initialize the interpreter.
    // Set the recursion limit down from its default, or the interpreter recursion
    // limit test will fail (the Rust stack will overflow).
    type YourCtx = ();
    let mut interp = InterpBuilder::new(
        (YourCtx::default(), TestCtx::new()),
        gen_command!(
            (YourCtx, TestCtx),
            [(_SOURCE, cmd_source), (_EXIT, cmd_exit), (_PARSE, cmd_parse)],
            [("test", test_cmd, "run a test case")]
        ),
    )
    .environment(true)
    .name("molt-test")
    .standard_library(if cfg!(feature = "full") {
        StandardLibrary::Full
    } else {
        StandardLibrary::Slim
    })
    .build();
    interp
        .set_scalar("molt_full", Value::from(cfg!(feature = "full")))
        .unwrap();
    // Keep this below the default test-thread stack budget. Until the 0.6 VM milestone
    // replaces recursive Rust evaluation frames, a debug full build has materially larger
    // frames than the slim 0.5 runtime.
    interp.set_recursion_limit(100);

    let args = vec![String::from("tests/all.tcl")];

    assert!(test_harness(&mut interp, &args).is_ok());
}
