fn contains_operator(value: &Sexp, operators: &[&str]) -> bool {
    if value.head().is_some_and(|head| operators.contains(&head)) {
        return true;
    }
    value
        .list()
        .is_some_and(|items| items.iter().any(|item| contains_operator(item, operators)))
}

fn contains_name(value: &Sexp, name: &str) -> bool {
    match value {
        Sexp::Name(value) => value == name,
        Sexp::List(items) => items.iter().any(|item| contains_name(item, name)),
        _ => false,
    }
}

fn section_head(section: &Sexp) -> Option<&str> {
    section.head()
}

fn strip_ppddl_requirements(section: &mut Sexp) {
    if section_head(section) != Some(":REQUIREMENTS") {
        return;
    }
    if let Some(items) = section.list_mut() {
        items.retain(|item| {
            let requirement = item.name();
            requirement != Some(PROB_REQ)
                && requirement != Some(REWARD_REQ)
                && requirement != Some(":MDP")
        });
    }
}
