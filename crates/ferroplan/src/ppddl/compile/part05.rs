fn validate_reserved_symbols(root: &Sexp) -> Result<(), PpddlError> {
    let items = define_items(root, "domain")?;
    for section in items.iter().skip(2) {
        match section_head(section) {
            Some(":PREDICATES") => {
                for predicate in section.list().unwrap_or_default().iter().skip(1) {
                    if let Some(name) = predicate.list().and_then(|items| items.first()).and_then(Sexp::name) {
                        if name == INIT_PENDING || name.starts_with(MARKER_PREFIX) {
                            return Err(PpddlError::Unsupported(format!(
                                "predicate {name} uses Ferroplan's reserved PPDDL namespace"
                            )));
                        }
                    }
                }
            }
            Some(":ACTION") => {
                if let Some(name) = section.list().and_then(|items| items.get(1)).and_then(Sexp::name) {
                    if name == INIT_ACTION || name.starts_with(VARIANT_PREFIX) {
                        return Err(PpddlError::Unsupported(format!(
                            "action {name} uses Ferroplan's reserved PPDDL namespace"
                        )));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn normalize_requirements(output: &mut Vec<Sexp>) {
    if let Some(section) = output
        .iter_mut()
        .find(|section| section_head(section) == Some(":REQUIREMENTS"))
    {
        strip_ppddl_requirements(section);
        let items = section.list_mut().expect("requirements is a list");
        if !items
            .iter()
            .any(|item| item.name() == Some(":NEGATIVE-PRECONDITIONS"))
        {
            items.push(Sexp::Name(":NEGATIVE-PRECONDITIONS".into()));
        }
    } else {
        output.push(Sexp::List(vec![
            Sexp::Name(":REQUIREMENTS".into()),
            Sexp::Name(":STRIPS".into()),
            Sexp::Name(":NEGATIVE-PRECONDITIONS".into()),
        ]));
    }
}
