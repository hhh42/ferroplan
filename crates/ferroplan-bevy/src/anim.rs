//! Solve off-thread and animate the plan trace: a timeline scrubbed by keyboard,
//! with mobiles tweened between the node they're on in successive snapshots.
//!
//! Controls: **S** solve · **Space** play/pause · **←/→** step · **R** reset.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use futures_lite::future;

use ferroplan::{Mode, Options, StateSnapshot, Step};

use crate::scene::{FanOffset, MobileObj, NodeObj, Scene};

struct SolveResult {
    steps: Vec<Step>,
    snapshots: Vec<StateSnapshot>,
    status: String,
}

#[derive(Resource, Default)]
pub struct Plan {
    pub steps: Vec<Step>,
    pub snapshots: Vec<StateSnapshot>,
    pub t: f32,
    pub playing: bool,
    pub status: String,
}

#[derive(Resource, Default)]
pub struct SolveJob(Option<Task<SolveResult>>);

pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    scene: Res<Scene>,
    editor: Res<crate::blocks::Editor>,
    mut plan: ResMut<Plan>,
    mut job: ResMut<SolveJob>,
) {
    // Don't steal keystrokes while the editor is capturing text.
    if editor.focus.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyS)
        && job.0.is_none()
        && !scene.domain_src.is_empty()
        && !scene.problem_src.is_empty()
    {
        let d = scene.domain_src.clone();
        let p = scene.problem_src.clone();
        job.0 = Some(AsyncComputeTaskPool::get().spawn(async move { solve_blocking(d, p) }));
        plan.status = "solving…".into();
    }
    let n = plan.steps.len() as f32;
    if keys.just_pressed(KeyCode::Space) && !plan.steps.is_empty() {
        if plan.t >= n {
            plan.t = 0.0;
        }
        plan.playing = !plan.playing;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        plan.t = (plan.t.floor() + 1.0).min(n);
        plan.playing = false;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        plan.t = (plan.t.floor() - 1.0).max(0.0);
        plan.playing = false;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        plan.t = 0.0;
        plan.playing = false;
    }
}

fn solve_blocking(domain: String, problem: String) -> SolveResult {
    match ferroplan::solve(&domain, &problem, &Options::default()) {
        Ok(sol) => match sol.plan {
            Some(plan) => {
                let pairs: Vec<(String, Vec<String>)> = plan
                    .steps
                    .iter()
                    .map(|s| (s.action.clone(), s.args.clone()))
                    .collect();
                let snapshots = if sol.mode == Mode::Temporal {
                    Vec::new()
                } else {
                    ferroplan::trace(&domain, &problem, &pairs).unwrap_or_default()
                };
                let mut status = format!("solved: {} steps", plan.steps.len());
                if let Some(m) = plan.metric {
                    status.push_str(&format!(", metric {m}"));
                }
                if sol.mode == Mode::Temporal {
                    status.push_str(" (temporal: animation n/a)");
                }
                SolveResult {
                    steps: plan.steps,
                    snapshots,
                    status,
                }
            }
            None => SolveResult {
                steps: vec![],
                snapshots: vec![],
                status: "no plan found".into(),
            },
        },
        Err(e) => SolveResult {
            steps: vec![],
            snapshots: vec![],
            status: format!("error: {e}"),
        },
    }
}

pub fn poll_solve(mut job: ResMut<SolveJob>, mut plan: ResMut<Plan>) {
    if let Some(task) = job.0.as_mut() {
        if let Some(res) = block_on(future::poll_once(task)) {
            job.0 = None;
            plan.steps = res.steps;
            plan.snapshots = res.snapshots;
            plan.status = res.status;
            plan.t = 0.0;
            plan.playing = false;
        }
    }
}

/// Baseline playback rate, in (unit-duration) steps per second.
const PLAY_RATE: f32 = 1.5;

pub fn advance(time: Res<Time>, mut plan: ResMut<Plan>) {
    if plan.playing && !plan.steps.is_empty() {
        let n = plan.steps.len() as f32;
        // Per-step-duration timing: the playhead dwells on each step in proportion
        // to that step's `duration` (temporal plans), so a 4s action takes 4× as
        // long on screen as a 1s one. Plain STRIPS steps have no duration → 1.0,
        // i.e. uniform playback as before.
        let k = (plan.t.floor() as usize).min(plan.steps.len() - 1);
        let dur = plan.steps[k].duration.unwrap_or(1.0).max(0.05) as f32;
        plan.t = (plan.t + time.delta_secs() * PLAY_RATE / dur).min(n);
        if plan.t >= n {
            plan.playing = false;
        }
    }
}

/// Move each mobile to its position for the current timeline `t`, tweening between
/// the node it's on in snapshot k and k+1.
pub fn animate(
    plan: Res<Plan>,
    scene: Res<Scene>,
    nodes: Query<(&NodeObj, &Transform)>,
    mut mobiles: Query<(&MobileObj, &FanOffset, &mut Transform), Without<NodeObj>>,
) {
    if plan.snapshots.is_empty() {
        return;
    }
    let count = plan.snapshots.len();
    let k = (plan.t.floor() as usize).min(count - 1);
    let kn = (k + 1).min(count - 1);
    let frac = if kn == k {
        0.0
    } else {
        // ease-in-out-cubic on the step-local progress (the redesign's motion curve),
        // so mobiles accelerate out of a node and settle into the next.
        ease_in_out_cubic((plan.t - k as f32).clamp(0.0, 1.0))
    };
    let from = scene.graph.positions_at(&plan.snapshots[k].facts);
    let to = scene.graph.positions_at(&plan.snapshots[kn].facts);
    let npos: HashMap<&str, Vec2> = nodes
        .iter()
        .map(|(n, t)| (n.0.as_str(), t.translation.truncate()))
        .collect();

    for (m, off, mut tf) in &mut mobiles {
        let here = tf.translation.truncate() - off.0;
        let fp = node_pos(&from, &m.0, &npos).unwrap_or(here);
        let tp = node_pos(&to, &m.0, &npos).unwrap_or(here);
        let target = fp.lerp(tp, frac) + off.0;
        tf.translation.x = target.x;
        tf.translation.y = target.y;
    }
}

/// Ease-in-out-cubic — smooth acceleration then deceleration over `t` in `0..=1`.
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn node_pos(
    map: &HashMap<String, Option<String>>,
    obj: &str,
    npos: &HashMap<&str, Vec2>,
) -> Option<Vec2> {
    map.get(obj)
        .and_then(|o| o.as_deref())
        .and_then(|n| npos.get(n).copied())
}
