# Evaluated unchanged by Tcl 8.6.18 and Molt. The caller places the case in the environment so
# neither interpreter needs a host-specific file/channel command.
set script $env(MOLT_DIFF_SCRIPT)
set complete [info complete $script]
set code [catch $script result options]
puts [list $complete $code $result $options]
