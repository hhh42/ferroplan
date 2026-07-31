fn parse_typed_items(items: &[Sexp]) -> Result<Vec<(String, Vec<String>)>, PpddlError> {
    let mut output = Vec::new();
    let mut pending = Vec::new();
    let mut index = 0;
    while index < items.len() {
        if matches!(items[index], Sexp::Dash) {
            index += 1;
            let ty = items
                .get(index)
                .ok_or_else(|| PpddlError::Syntax("typed list ends after '-'".into()))?;
            let accepted = type_names(ty)?;
            for name in pending.drain(..) {
                output.push((name, accepted.clone()));
            }
        } else if let Some(name) = typed_item_name(&items[index]) {
            pending.push(name);
        } else {
            return Err(PpddlError::Syntax(
                "unexpected item in typed list".into(),
            ));
        }
        index += 1;
    }
    for name in pending {
        output.push((name, vec!["OBJECT".into()]));
    }
    Ok(output)
}

fn substitute(value: &Sexp, binding: &HashMap<String, String>) -> Sexp {
    match value {
        Sexp::Var(variable) => binding
            .get(variable)
            .cloned()
            .map(Sexp::Name)
            .unwrap_or_else(|| value.clone()),
        Sexp::List(items) => {
            Sexp::List(items.iter().map(|item| substitute(item, binding)).collect())
        }
        _ => value.clone(),
    }
}

fn product_outcomes(
    left: &[(f64, Sexp)],
    right: &[(f64, Sexp)],
    limit: usize,
    context: &str,
) -> Result<Vec<(f64, Sexp)>, PpddlError> {
    let mut product = Vec::new();
    for (left_probability, left_effect) in left {
        for (right_probability, right_effect) in right {
            product.push((
                left_probability * right_probability,
                join_effects(left_effect, right_effect),
            ));
            if product.len() > limit {
                return Err(PpddlError::OutcomeLimit {
                    action: context.into(),
                    limit,
                });
            }
        }
    }
    Ok(product)
}
