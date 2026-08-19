# Test Script: apply

test apply-1.1 {required and default arguments} {
    apply {{x {y 2}} {expr {$x + $y}}} 3
} -ok 5

test apply-1.2 {variadic arguments} {
    apply {{args} {llength $args}} a b c
} -ok 3

test apply-1.3 {root namespace field} {
    apply {{x} {set x} ::} value
} -ok value

test apply-1.4 {anonymous local scope is restored after an argument error} {
    catch {apply {{x} {set leaked yes}}}
    info exists leaked
} -ok 0

test apply-1.5 {invalid lambda list length} {
    apply {x}
} -error {can't interpret "x" as a lambda expression}

test apply-1.6 {unsupported namespace is reported} {
    apply {{x} {set x} ::missing} value
} -error {namespace "::missing" not found}
