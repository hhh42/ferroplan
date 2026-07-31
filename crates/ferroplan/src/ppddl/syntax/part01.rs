#[derive(Clone, Debug)]
enum Sexp {
    Name(String),
    Var(String),
    Num(f64),
    Op(String),
    Dash,
    List(Vec<Sexp>),
}

impl Sexp {
    fn name(&self) -> Option<&str> {
        match self {
            Self::Name(value) => Some(value),
            _ => None,
        }
    }

    fn list(&self) -> Option<&[Sexp]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    fn list_mut(&mut self) -> Option<&mut Vec<Sexp>> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    fn head(&self) -> Option<&str> {
        self.list()?.first()?.name()
    }
}

fn parse_sexp(src: &str) -> Result<Sexp, PpddlError> {
    let (tokens, _) = lex(src).map_err(|error| PpddlError::Syntax(error.to_string()))?;
    let mut index = 0;
    let value = parse_sexp_at(&tokens, &mut index)?;
    if index != tokens.len() {
        return Err(PpddlError::Syntax(
            "trailing tokens after PPDDL document".into(),
        ));
    }
    Ok(value)
}

fn parse_sexp_at(tokens: &[Tok], index: &mut usize) -> Result<Sexp, PpddlError> {
    let token = tokens
        .get(*index)
        .ok_or_else(|| PpddlError::Syntax("unexpected end of PPDDL input".into()))?;
    *index += 1;
    match token {
        Tok::LParen => {
            let mut items = Vec::new();
            while !matches!(tokens.get(*index), Some(Tok::RParen)) {
                if *index >= tokens.len() {
                    return Err(PpddlError::Syntax("unclosed PPDDL list".into()));
                }
                items.push(parse_sexp_at(tokens, index)?);
            }
            *index += 1;
            Ok(Sexp::List(items))
        }
        Tok::RParen => Err(PpddlError::Syntax("unexpected ')'".into())),
        Tok::Name(value) => Ok(Sexp::Name(value.clone())),
        Tok::Var(value) => Ok(Sexp::Var(value.clone())),
        Tok::Num(value) => Ok(Sexp::Num(*value)),
        Tok::Op(value) => Ok(Sexp::Op(value.clone())),
        Tok::Dash => Ok(Sexp::Dash),
    }
}

fn render_number(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    let mut rendered = format!("{value:.15}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.pop();
    }
    rendered
}

fn render(value: &Sexp) -> String {
    match value {
        Sexp::Name(value) | Sexp::Op(value) => value.clone(),
        Sexp::Var(value) => format!("?{value}"),
        Sexp::Num(value) => render_number(*value),
        Sexp::Dash => "-".into(),
        Sexp::List(items) => format!(
            "({})",
            items.iter().map(render).collect::<Vec<_>>().join(" ")
        ),
    }
}

fn atom(name: impl Into<String>) -> Sexp {
    Sexp::List(vec![Sexp::Name(name.into())])
}

fn noop_effect() -> Sexp {
    Sexp::List(vec![Sexp::Name("AND".into())])
}

fn effect_parts(effect: &Sexp) -> Vec<Sexp> {
    if effect.head() == Some("AND") {
        effect
            .list()
            .unwrap_or_default()
            .iter()
            .skip(1)
            .cloned()
            .collect()
    } else {
        vec![effect.clone()]
    }
}

fn join_effects(left: &Sexp, right: &Sexp) -> Sexp {
    let mut items = vec![Sexp::Name("AND".into())];
    items.extend(effect_parts(left));
    items.extend(effect_parts(right));
    Sexp::List(items)
}

fn conjunction(effects: impl IntoIterator<Item = Sexp>) -> Sexp {
    let mut items = vec![Sexp::Name("AND".into())];
    for effect in effects {
        items.extend(effect_parts(&effect));
    }
    Sexp::List(items)
}

fn numeric_constant(value: &Sexp) -> Result<f64, PpddlError> {
    let number = match value {
        Sexp::Num(number) => *number,
        Sexp::List(items) if items.len() == 2 && matches!(&items[0], Sexp::Dash) => {
            match &items[1] {
                Sexp::Num(number) => -*number,
                _ => {
                    return Err(PpddlError::Syntax(
                        "negative numeric constant must contain a number".into(),
                    ))
                }
            }
        }
        Sexp::List(items)
            if items.len() == 3
                && matches!(items.first(), Some(Sexp::Op(operator)) if operator == "/") =>
        {
            let numerator = match &items[1] {
                Sexp::Num(number) => *number,
                _ => {
                    return Err(PpddlError::Syntax(
                        "numeric division numerator must be a number".into(),
                    ))
                }
            };
            let denominator = match &items[2] {
                Sexp::Num(number) => *number,
                _ => {
                    return Err(PpddlError::Syntax(
                        "numeric division denominator must be a number".into(),
                    ))
                }
            };
            if denominator == 0.0 {
                return Err(PpddlError::Syntax(
                    "numeric division denominator is zero".into(),
                ));
            }
            numerator / denominator
        }
        _ => {
            return Err(PpddlError::Syntax(
                "expected a finite numeric constant".into(),
            ))
        }
    };
    if !number.is_finite() {
        return Err(PpddlError::Syntax(
            "numeric constant must be finite".into(),
        ));
    }
    Ok(number)
}
