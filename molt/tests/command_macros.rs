extern crate molt_forked as renamed_molt;

use renamed_molt::prelude::*;

fn cmd_leaf(_interp: &mut Interp<()>, argv: &[Value]) -> MoltResult {
    molt_ok!(argv.last().unwrap().clone())
}

fn cmd_about(_interp: &mut Interp<()>, _argv: &[Value]) -> MoltResult {
    molt_ok!("about")
}

#[test]
fn subcommands_are_statically_dispatched_and_aligned() {
    let command = renamed_molt::gen_subcommand!(
        (),
        1,
        [("短", cmd_leaf, "wide"), ("abc", cmd_leaf, "ascii")],
    );
    let mut interp = Interp::default();

    let help = command(&mut interp, &["root".into(), "-help".into()]).unwrap();
    assert_eq!(help.as_str(), "usage of root:\n  短     wide\n  abc    ascii\n  -help");

    let result = command(&mut interp, &["root".into(), "abc".into()]).unwrap();
    assert_eq!(result.as_str(), "abc");

    let error = command(&mut interp, &["root".into(), "missing".into()]).unwrap_err();
    assert!(error
        .value()
        .as_str()
        .starts_with("unknown subcommand in \"root missing\""));
}

#[test]
fn nested_subcommand_index_preserves_the_command_prefix() {
    let command = renamed_molt::gen_subcommand!(
        (),
        2,
        [("leaf", cmd_leaf, "run the nested command")],
    );
    let mut interp = Interp::default();
    let help =
        command(&mut interp, &["root".into(), "branch".into(), "-help".into()]).unwrap();
    assert!(help.as_str().starts_with("usage of root branch:\n"));
}

#[test]
fn generated_command_supports_help_unknown_and_embedded_dispatch() {
    let command = renamed_molt::gen_command!(
        (),
        [],
        [("about", cmd_about, "display app information")],
    );
    let mut interp = InterpBuilder::new((), command).name("macro-test").build();

    assert_eq!(interp.eval("about").unwrap().as_str(), "about");
    assert_eq!(
        interp.eval("help").unwrap().as_str(),
        "usage of macro-test:\n  about  display app information\n  help   [-all]"
    );

    let error = interp.eval("missing").unwrap_err();
    assert!(error.value().as_str().starts_with("unknown command \"missing\""));
}
