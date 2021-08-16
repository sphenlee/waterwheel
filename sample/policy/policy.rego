package waterwheel

default authorize = false

authorize {
    is_readonly
}

authorize {
    input.http["x-waterwheel-user"] == "admin"
}

is_readonly {
    input.action == "Get"
}
is_readonly {
    input.action == "List"
}
