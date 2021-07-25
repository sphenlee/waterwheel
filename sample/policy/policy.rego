package waterwheel

default allow = false

allow {
    input.action == "Get"
}

allow {
    input.principal.identity = "admin"
}

test_project := "4fb168e4-b963-431d-85b6-ac108542c036"
autogen_project := "9496b838-f77b-410e-9d67-72a320cb00e0"

project_members := {
    test_project: ["user1"],
    autogen_project: ["user1", "user2"],
}

allow {
    users := project_members[input.object.project_id]

    input.principal.identity = users[_]
    input.principal.authority = "bearer"
}

