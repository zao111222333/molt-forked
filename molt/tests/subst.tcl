# Tcl 8.6 subst command.

test subst-1.1 {performs all three substitution classes} {
    set value X
    subst {$value [list a b] \n}
} -ok "X a b \n"

test subst-1.2 {-nocommands preserves brackets but applies other substitutions} {
    set value X
    subst -nocommands {$value [list a b] \n}
} -ok "X \[list a b\] \n"

test subst-1.3 {-novariables preserves variable syntax} {
    set value X
    subst -novariables {$value [list a b]}
} -ok {$value a b}

test subst-1.4 {-nobackslashes preserves escape syntax} {
    set value X
    subst -nobackslashes {$value [list a b] \n}
} -ok {X a b \n}

test subst-1.5 {break and continue have Tcl subst control semantics} {
    list [subst {before[break]after}] [subst {a[continue]b}] [subst {a[return x]b}]
} -ok {before ab axb}

test subst-1.6 {invalid options are rejected} {
    subst -bad value
} -error {bad option "-bad": must be -nobackslashes, -nocommands, or -novariables}
