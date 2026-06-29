//! A timescale (Gantt) view for temporal plans: each durative action is a bar on
//! a shared plan-time axis, packed into lanes so non-overlapping actions share a
//! row, with a cyan "now" line swept by the transport playhead. This is how a
//! temporal plan — overlapping durative actions the graph can't tween — is made
//! legible. Shown automatically when a temporal plan is loaded; toggle with **T**.

use bevy::prelude::*;

use crate::anim::Plan;
use crate::icons;

#[derive(Resource)]
pub struct GanttState {
    pub open: bool,
    built_for: usize,
    built_span: f32,
}

impl Default for GanttState {
    fn default() -> Self {
        Self {
            open: true,
            built_for: usize::MAX,
            built_span: -1.0,
        }
    }
}

#[derive(Component)]
pub struct GanttPanel;
#[derive(Component)]
pub struct GanttTrack;
#[derive(Component)]
pub struct GanttBar;
#[derive(Component)]
pub struct GanttNow;

/// Beyond this many actions the per-bar labels are dropped (the bars still draw).
const LABEL_LIMIT: usize = 40;

pub fn setup_gantt(mut commands: Commands) {
    commands
        .spawn((
            GanttPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(340.0), // clear the inspector
                bottom: Val::Px(54.0), // sit above the transport bar
                height: Val::Px(190.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(6.0),
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(crate::palette::PANEL_BLUR),
            BorderColor::all(crate::palette::EDGE2),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("TIMESCALE  ·  durative actions  ·  T to toggle"),
                TextFont {
                    font_size: 10.0_f32.into(),
                    ..default()
                },
                TextColor(crate::palette::FAINT),
            ));
            // the lane area; bars + the now-line are absolute children placed by
            // percentage of the makespan.
            p.spawn((
                GanttTrack,
                Node {
                    flex_grow: 1.0,
                    width: Val::Percent(100.0),
                    ..default()
                },
            ))
            .with_children(|t| {
                t.spawn((
                    GanttNow,
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        left: Val::Percent(0.0),
                        width: Val::Px(2.0),
                        ..default()
                    },
                    BackgroundColor(crate::palette::CY),
                ));
            });
        });
}

/// **T** toggles the timescale (only meaningful for temporal plans).
pub fn toggle_gantt(
    keys: Res<ButtonInput<KeyCode>>,
    editor: Res<crate::blocks::Editor>,
    mut state: ResMut<GanttState>,
) {
    if editor.focus.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        state.open = !state.open;
    }
}

pub fn gantt_visibility(
    plan: Res<Plan>,
    state: Res<GanttState>,
    editor: Res<crate::blocks::Editor>,
    mut panel: Query<&mut Visibility, With<GanttPanel>>,
) {
    let Ok(mut vis) = panel.single_mut() else {
        return;
    };
    let want = if plan.temporal && !plan.steps.is_empty() && state.open && !editor.open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *vis != want {
        *vis = want;
    }
}

/// Greedy lane packing: actions sorted by start time, each placed in the first
/// lane whose last action has already ended. Returns `(lane, step_index)` and the
/// total lane count.
fn pack_lanes(plan: &Plan) -> (Vec<(usize, usize)>, usize) {
    let mut order: Vec<usize> = (0..plan.steps.len()).collect();
    order.sort_by(|&a, &b| {
        let ta = plan.steps[a].time.unwrap_or(0.0);
        let tb = plan.steps[b].time.unwrap_or(0.0);
        ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut lane_end: Vec<f64> = Vec::new();
    let mut placed = Vec::with_capacity(order.len());
    for idx in order {
        let s = &plan.steps[idx];
        let start = s.time.unwrap_or(0.0);
        let end = start + s.duration.unwrap_or(0.0);
        let lane = lane_end
            .iter()
            .position(|&e| e <= start + 1e-6)
            .unwrap_or_else(|| {
                lane_end.push(0.0);
                lane_end.len() - 1
            });
        lane_end[lane] = end;
        placed.push((lane, idx));
    }
    let lanes = lane_end.len().max(1);
    (placed, lanes)
}

/// Rebuild the bars when the plan (length or makespan) changes.
#[allow(clippy::type_complexity)]
pub fn rebuild_gantt(
    mut commands: Commands,
    plan: Res<Plan>,
    mut state: ResMut<GanttState>,
    track: Query<Entity, With<GanttTrack>>,
    bars: Query<Entity, With<GanttBar>>,
) {
    let n = plan.steps.len();
    if !plan.temporal {
        // tear down any stale bars when leaving temporal mode
        if state.built_for != usize::MAX {
            for e in &bars {
                commands.entity(e).despawn();
            }
            state.built_for = usize::MAX;
            state.built_span = -1.0;
        }
        return;
    }
    if n == state.built_for && (plan.makespan - state.built_span).abs() < 1e-4 {
        return;
    }
    state.built_for = n;
    state.built_span = plan.makespan;
    for e in &bars {
        commands.entity(e).despawn();
    }
    let Ok(track) = track.single() else {
        return;
    };
    let span = plan.span();
    let (placed, lanes) = pack_lanes(&plan);
    let lane_h = 100.0 / lanes as f32;
    let with_labels = n <= LABEL_LIMIT;
    commands.entity(track).with_children(|t| {
        for (lane, idx) in placed {
            let s = &plan.steps[idx];
            let start = s.time.unwrap_or(0.0) as f32;
            let dur = s.duration.unwrap_or(0.0) as f32;
            let left = 100.0 * (start / span);
            let width = (100.0 * (dur / span)).max(0.6); // keep instantaneous acts visible
            let color = icons::color_for(s.args.first().map(|a| a.as_str()).unwrap_or(&s.action));
            t.spawn((
                GanttBar,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(lane as f32 * lane_h),
                    height: Val::Percent(lane_h),
                    left: Val::Percent(left),
                    width: Val::Percent(width),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                    margin: UiRect::all(Val::Px(1.0)),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(color.with_alpha(0.85)),
            ))
            .with_children(|b| {
                if with_labels {
                    b.spawn((
                        Text::new(s.action.to_lowercase()),
                        TextFont {
                            font_size: 9.0_f32.into(),
                            ..default()
                        },
                        TextColor(crate::palette::BG),
                    ));
                }
            });
        }
    });
}

/// Sweep the now-line to the current plan time.
pub fn gantt_now(plan: Res<Plan>, mut now: Query<&mut Node, With<GanttNow>>) {
    let Ok(mut node) = now.single_mut() else {
        return;
    };
    node.left = Val::Percent(plan.frac() * 100.0);
}
