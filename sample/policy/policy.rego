package waterwheel

default authorize = false

authorize {
    true
}

is_readonly {
    input.action == "Get"
}
is_readonly {
    input.action == "List"
}
