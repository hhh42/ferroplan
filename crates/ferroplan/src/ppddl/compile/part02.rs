fn validate_reward_effect(effect: &Sexp) -> Result<bool, PpddlError> {
    let Some(items) = effect.list() else {
        return Err(PpddlError::Syntax("effect must be a list".into()));
    };
    let Some(head) = items.first().and_then(Sexp::name) else {
        if contains_name(effect, "REWARD") {
            return Err(PpddlError::RewardViolation(
                "reward may only occur as an increase/decrease target".into(),
            ));
        }
        return Ok(false);
    };
    match head {
        "AND" | "ONEOF" => {
            let mut used = false;
            for child in &items[1..] {
                used |= validate_reward_effect(child)?;
            }
            Ok(used)
        }
        "PROBABILISTIC" => {
            if items.len() < 3 || items.len() % 2 == 0 {
                return Err(PpddlError::Syntax(
                    "probabilistic effect requires probability/effect pairs".into(),
                ));
            }
            let mut used = false;
            for pair in items[1..].chunks_exact(2) {
                used |= validate_reward_effect(&pair[1])?;
            }
            Ok(used)
        }
        "FORALL" => {
            if items.len() != 3 {
                return Err(PpddlError::Syntax(
                    "forall effect requires variables and effect".into(),
                ));
            }
            validate_reward_effect(&items[2])
        }
        "WHEN" => {
            if items.len() != 3 {
                return Err(PpddlError::Syntax(
                    "when effect requires condition and effect".into(),
                ));
            }
            if contains_name(&items[1], "REWARD") {
                return Err(PpddlError::RewardViolation(
                    "effect conditions may not reference reward".into(),
                ));
            }
            validate_reward_effect(&items[2])
        }
        "INCREASE" | "DECREASE" => {
            if items.len() != 3 {
                return Err(PpddlError::Syntax(format!(
                    "{head} effect requires target and expression"
                )));
            }
            if reward_target(&items[1]) {
                if contains_name(&items[2], "REWARD") {
                    return Err(PpddlError::RewardViolation(
                        "reward update expressions may not reference reward".into(),
                    ));
                }
                Ok(true)
            } else if contains_name(effect, "REWARD") {
                Err(PpddlError::RewardViolation(
                    "reward may only occur as an increase/decrease target".into(),
                ))
            } else {
                Ok(false)
            }
        }
        "ASSIGN" | "SCALE-UP" | "SCALE-DOWN" => {
            if contains_name(effect, "REWARD") {
                Err(PpddlError::RewardViolation(
                    "reward only supports increase and decrease".into(),
                ))
            } else {
                Ok(false)
            }
        }
        _ => {
            if contains_name(effect, "REWARD") {
                Err(PpddlError::RewardViolation(
                    "reward may only occur as an increase/decrease target".into(),
                ))
            } else {
                Ok(false)
            }
        }
    }
}

fn init_assignment_target(value: &Sexp) -> Option<&Sexp> {
    let items = value.list()?;
    if items.len() == 3 && matches!(items.first(), Some(Sexp::Op(operator)) if operator == "=") {
        items.get(1)
    } else {
        None
    }
}
