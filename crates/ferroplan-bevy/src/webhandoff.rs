//! Read the "Animate this plan" handoff the Solver web page (`ferroplan-wasm/web/`)
//! writes to `localStorage['ferroplan.handoff']` before navigating here — the domain,
//! problem, and the solution ALREADY SOLVED there, so clicking "Animate" shows that
//! exact plan instead of falling back to the embedded demo or re-solving (which could
//! disagree with the Solver's result if search options ever differ).
//!
//! wasm32-only; `main.rs` calls [`try_load`] at startup and falls back to the
//! embedded demo when it returns `false` (no handoff, or it failed to parse).

use bevy::prelude::*;

use crate::anim::{load_result, result_from_solution, Plan};
use crate::scene::Scene;

const KEY: &str = "ferroplan.handoff";

/// Read, parse, and apply the handoff if present. Returns `true` on success (the
/// scene + plan are populated and the caller should skip loading the demo).
pub(crate) fn try_load(scene: &mut Scene, plan: &mut Plan) -> bool {
    let Some(raw) = read_local_storage(KEY) else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        web_sys::console::warn_1(&"ferroplan.handoff: invalid JSON, ignoring".into());
        return false;
    };
    let (Some(domain), Some(problem)) = (
        v.get("domain").and_then(|x| x.as_str()),
        v.get("problem").and_then(|x| x.as_str()),
    ) else {
        web_sys::console::warn_1(&"ferroplan.handoff: missing domain/problem, ignoring".into());
        return false;
    };
    scene.load_src(domain);
    scene.load_src(problem);

    // The solved plan is optional in principle (a future caller might hand off just
    // a domain+problem to load and let the user press S) — apply it if present and
    // parses as a Solution; a plan-less handoff still counts as a successful load
    // since the scene came through.
    if let Some(sol_v) = v.get("solution") {
        match serde_json::from_value::<ferroplan::Solution>(sol_v.clone()) {
            Ok(sol) => {
                let res = result_from_solution(domain, problem, sol);
                load_result(plan, res, true); // autoplay: they clicked "Animate"
            }
            Err(e) => web_sys::console::warn_1(
                &format!("ferroplan.handoff: solution didn't parse ({e}), scene loaded anyway")
                    .into(),
            ),
        }
    }
    true
}

fn read_local_storage(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(key)
        .ok()?
}
