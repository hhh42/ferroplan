fn probability(value: &Sexp) -> Result<f64, PpddlError> {
    let number = numeric_constant(value)
        .map_err(|error| PpddlError::InvalidProbability(error.to_string()))?;
    if !(0.0..=1.0 + PROB_EPS).contains(&number) {
        return Err(PpddlError::InvalidProbability(format!(
            "probability {number} is outside [0, 1]"
        )));
    }
    Ok(number.clamp(0.0, 1.0))
}

#[derive(Clone, Debug, Default)]
struct ObjectUniverse {
    parent: HashMap<String, String>,
    objects: Vec<(String, String)>,
}

impl ObjectUniverse {
    fn from_documents(domain: &Sexp, problem: &Sexp) -> Result<Self, PpddlError> {
        let mut universe = Self::default();
        let domain_items = define_items(domain, "domain")?;
        for section in domain_items.iter().skip(2) {
            match section_head(section) {
                Some(":TYPES") => {
                    let items = section.list().unwrap_or_default();
                    for (name, types) in parse_typed_items(&items[1..])? {
                        let parent = types.first().cloned().unwrap_or_else(|| "OBJECT".into());
                        if name != parent {
                            universe.parent.insert(name, parent);
                        }
                    }
                }
                Some(":CONSTANTS") => {
                    let items = section.list().unwrap_or_default();
                    for (name, types) in parse_typed_items(&items[1..])? {
                        universe
                            .objects
                            .push((name, types.first().cloned().unwrap_or_else(|| "OBJECT".into())));
                    }
                }
                _ => {}
            }
        }
        let problem_items = define_items(problem, "problem")?;
        for section in problem_items.iter().skip(2) {
            if section_head(section) == Some(":OBJECTS") {
                let items = section.list().unwrap_or_default();
                for (name, types) in parse_typed_items(&items[1..])? {
                    universe
                        .objects
                        .push((name, types.first().cloned().unwrap_or_else(|| "OBJECT".into())));
                }
            }
        }
        universe.objects.sort();
        universe.objects.dedup();
        Ok(universe)
    }

    fn is_subtype(&self, child: &str, parent: &str) -> bool {
        if parent == "OBJECT" || child == parent {
            return true;
        }
        let mut current = child;
        for _ in 0..=self.parent.len() {
            let Some(next) = self.parent.get(current) else {
                return false;
            };
            if next == parent {
                return true;
            }
            current = next;
        }
        false
    }

    fn matching(&self, accepted: &[String]) -> Vec<String> {
        self.objects
            .iter()
            .filter(|(_, ty)| accepted.iter().any(|want| self.is_subtype(ty, want)))
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn bindings(
        &self,
        variables: &[(String, Vec<String>)],
    ) -> Vec<HashMap<String, String>> {
        let mut bindings = vec![HashMap::new()];
        for (variable, accepted) in variables {
            let candidates = self.matching(accepted);
            let mut next = Vec::new();
            for binding in &bindings {
                for object in &candidates {
                    let mut extended = binding.clone();
                    extended.insert(variable.clone(), object.clone());
                    next.push(extended);
                }
            }
            bindings = next;
        }
        bindings
    }
}

fn define_items<'a>(document: &'a Sexp, kind: &str) -> Result<&'a [Sexp], PpddlError> {
    let items = document
        .list()
        .ok_or_else(|| PpddlError::Syntax(format!("{kind} root must be a list")))?;
    if items.first().and_then(Sexp::name) != Some("DEFINE") {
        return Err(PpddlError::Syntax(format!(
            "expected (define ...) {kind}"
        )));
    }
    Ok(items)
}

fn type_names(value: &Sexp) -> Result<Vec<String>, PpddlError> {
    match value {
        Sexp::Name(name) => Ok(vec![name.clone()]),
        Sexp::List(items) if items.first().and_then(Sexp::name) == Some("EITHER") => {
            let names = items[1..]
                .iter()
                .map(|item| {
                    item.name()
                        .map(str::to_string)
                        .ok_or_else(|| PpddlError::Syntax("either type must contain names".into()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if names.is_empty() {
                return Err(PpddlError::Syntax("either type is empty".into()));
            }
            Ok(names)
        }
        _ => Err(PpddlError::Syntax("expected a type name".into())),
    }
}

fn typed_item_name(value: &Sexp) -> Option<String> {
    match value {
        Sexp::Name(name) | Sexp::Var(name) => Some(name.clone()),
        _ => None,
    }
}
