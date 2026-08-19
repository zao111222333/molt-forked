# Tcl 8.6 upvar/uplevel scope compatibility.

proc upvar_set {remote value} {
    upvar 1 $remote local
    set local $value
}

test upvar-1.1 {links differently named variables in the caller} {
    set target old
    upvar_set target new
    set target
} -ok {new}

proc upvar_unset {remote} {
    upvar 1 $remote local
    unset local
    set local recreated
}

test upvar-1.2 {unset removes the target but preserves the local link} {
    set target old
    upvar_unset target
    set target
} -ok {recreated}

proc same_frame_alias {} {
    set original first
    upvar 0 original alias
    set alias second
    set original
}

test upvar-1.3 {relative level zero aliases within one frame} {
    same_frame_alias
} -ok {second}

proc uplevel_inner {} {
    uplevel 1 {set value changed}
}

proc uplevel_outer {} {
    set value original
    uplevel_inner
    return $value
}

test uplevel-1.1 {evaluates a script in the caller frame} {
    uplevel_outer
} -ok {changed}

proc uplevel_helper {} {
    return helper-result
}

proc uplevel_calls_proc {} {
    uplevel 1 uplevel_helper
}

test uplevel-1.2 {a procedure called from uplevel restores both frames} {
    list [uplevel_calls_proc] [uplevel_calls_proc]
} -ok {helper-result helper-result}

proc uplevel_error {} {
    catch {uplevel 1 {error expected}}
    set local still-alive
    return $local
}

test uplevel-1.3 {errors restore the active scope} {
    uplevel_error
} -ok {still-alive}

rename upvar_set {}
rename upvar_unset {}
rename same_frame_alias {}
rename uplevel_inner {}
rename uplevel_outer {}
rename uplevel_helper {}
rename uplevel_calls_proc {}
rename uplevel_error {}
