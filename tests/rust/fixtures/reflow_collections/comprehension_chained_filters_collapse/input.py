eligible = [
    person
    for person in applicants
    if person.is_active
    if person.has_quorum
]
