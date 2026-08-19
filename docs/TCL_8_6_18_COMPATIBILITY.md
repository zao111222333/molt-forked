# Tcl 8.6.18 compatibility

Molt 0.6 separates the portable Tcl language from host services. The `full` feature is the
compatibility target; the default feature set remains the small embedding runtime. This matrix is
also the release gate: a row may only be marked **complete** after the corresponding upstream
Tcl 8.6.18 tests pass without an unlisted skip.

## Portable language matrix

| Area | Slim | `full` | Status in this tree |
| --- | --- | --- | --- |
| Script/list tokenization, substitutions, `{*}` expansion | included | included | shared analysis API complete; execution parser convergence in progress |
| Lossless syntax tokens and complete/incomplete/invalid status | included | included | complete |
| Fixed-width expressions and short-circuit/ternary operators | included | included | complete for implemented operators |
| Arbitrary-precision expression integers | — | included | implemented |
| Core variables, procedures, `upvar`, `uplevel` | included | included | implemented; namespaces and traces pending |
| List commands, multi-list `foreach`, `lmap` | included | included | substantially implemented |
| String commands and Tcl glob matching | included | included | substantially implemented; Unicode class edge cases remain |
| Dictionaries and arrays | included | included | common 8.6 operations implemented; iterator/update forms pending |
| `apply`, `eval`, `subst`, `switch`, `try` | included | included | implemented except regexp-backed switch modes |
| Tcl ARE regular expressions | — | required | pending; Rust `regex` is deliberately not substituted |
| Bytearray, binary and encoding commands | representation only | required | pending |
| Namespace, trace and child interpreters | — | required | pending |
| Coroutines, tailcall and explicit VM frames | — | required | pending |
| TclOO, package, history, clock, zlib, mathfunc/mathop, `tcl::prefix` | — | required | pending |

The enhanced Molt command/subcommand usage and `-help` output are an intentional compatibility
exception. Unknown application commands receive lexical highlighting only.

## Host-capability exclusions

The following Tcl command families are outside the portable 0.6 score. They may be supplied by an
embedding application and are tracked in `compat/tcl86-host-exclusions.tsv`:

- filesystem and process commands;
- sockets, channels and the event loop;
- dynamic loading and Tcl's C ABI;
- Tk and packages that require native host services.

Molt's existing `source`, `exit`, environment import and standard-output buffer remain optional
host extensions. A skipped upstream test is invalid unless its file or constraint is recorded in
the exclusion table with a reason.

## Frozen upstream baseline

The unmodified Tcl 8.6.18 test directory, `tcltest` package and upstream license are stored under
`vendor/tcl8.6.18`. `vendor/tcl8.6.18/MOLT_MANIFEST.md` records the release archive and SHA-256.
Normal CI never downloads this data.

`tools/tcl-diff.sh path/to/case.tcl` compares parser completeness, completion code, result and
return options with an installed `tclsh8.6.18`. Set `TCLSH=/path/to/tclsh8.6.18` when it is not on
`PATH`. This differential tool is diagnostic; the checked-in regression tests remain the
deterministic release gate.
