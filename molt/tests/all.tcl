# Runs all tests.
#
# If I add the "glob" command, I'll use that to pick up the tests.

# Embedders set this marker to match their selected standard-library profile. Keep the historical
# full default for direct sourcing by applications that do not provide it.
if {![info exists molt_full]} {
    set molt_full 1
}

source append.tcl
source array.tcl
source assert_eq.tcl
source break.tcl
source catch.tcl
source continue.tcl
source dict.tcl
source error.tcl
source exit.tcl
source expr.tcl
source for.tcl
source foreach.tcl
source if.tcl
source info.tcl
source incr.tcl
source interp.tcl
source join.tcl
source lappend.tcl
source lindex.tcl
source list.tcl
source llength.tcl
source parser.tcl
source proc.tcl
source rename.tcl
source return.tcl
source set.tcl
source string.tcl
source test.tcl
source throw.tcl
source unset.tcl
source while.tcl

if {$molt_full} {
    source apply.tcl
    source list86.tcl
    source subst.tcl
    source switch.tcl
    source try.tcl
    source upvar.tcl
}
