# Test Script: switch

test switch-1.1 {exact and default arms} {
    list \
        [switch a a {set x yes} default {set x no}] \
        [switch z a {set x yes} default {set x no}]
} -ok {yes no}

test switch-1.2 {glob and nocase modes} {
    list \
        [switch -glob abc a* {set x yes} default {set x no}] \
        [switch -nocase A a {set x yes} default {set x no}]
} -ok {yes yes}

test switch-1.3 {list form and fallthrough body} {
    switch c {a - b - c {set x yes} default {set x no}}
} -ok yes

test switch-1.4 {default only has special meaning in final position} {
    switch z default {set x bad} z {set x yes}
} -ok yes

test switch-1.5 {no matching arm returns empty value} {
    switch z a {set x no}
} -ok {}

test switch-1.6 {odd pattern/body list is rejected} {
    switch a a
} -error {extra switch pattern with no body}
