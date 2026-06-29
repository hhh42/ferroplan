//! A bottom-docked transport bar for the plan animation: a play/pause button, a
//! scrubbable timeline (click or drag to seek) with one notch per step, a molten
//! progress fill + playhead, and a step/time readout. It mirrors the keyboard
//! controls (Space / ←→ / R) so the animator is usable with the mouse alone.
//!
//! The bar only shows while a plan with steps is loaded. The track reports the
//! pointer's normalized position via `RelativeCursorPosition`, which we map to a
//! timeline `t` on press/drag.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::anim::Plan;

/// True while the pointer is over the transport bar — lets world interaction
/// (node selection) ignore clicks that are really scrubbing.
#[derive(Resource, Default)]
pub struct Transport {
    pub hovering: bool,
    /// Step count the notches were last built for (so we only rebuild on change).
    built_for: usize,
}

#[derive(Component)]
pub struct TransportBar;
#[derive(Component)]
pub struct PlayButton;
#[derive(Component)]
pub struct PlayIcon;
#[derive(Component)]
pub struct ScrubTrack;
#[derive(Component)]
pub struct ScrubFill;
#[derive(Component)]
pub struct Playhead;
#[derive(Component)]
pub struct StepNotch;
#[derive(Component)]
pub struct TransportLabel;

/// One notch per step is drawn while the plan is short enough to read; denser
/// plans rely on the fill + playhead alone.
const MAX_NOTCHES: usize = 80;

pub fn setup_transport(mut commands: Commands) {
    commands
        .spawn((
            TransportBar,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(340.0), // clear the inspector panel
                bottom: Val::Px(0.0),
                height: Val::Px(54.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(crate::palette::PANEL_BLUR),
            BorderColor::all(crate::palette::EDGE2),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            // play / pause button
            p.spawn((
                PlayButton,
                Button,
                Node {
                    width: Val::Px(34.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(crate::palette::PANEL2),
            ))
            .with_children(|b| {
                b.spawn((
                    PlayIcon,
                    Text::new("\u{25B6}"), // ▶
                    TextFont {
                        font_size: 14.0_f32.into(),
                        ..default()
                    },
                    TextColor(crate::palette::INK),
                ));
            });

            // scrub track (grows to fill); fill, notches and playhead are absolute
            // children positioned by percentage of `t / n`.
            p.spawn((
                ScrubTrack,
                Button, // so it reports Interaction
                RelativeCursorPosition::default(),
                Node {
                    flex_grow: 1.0,
                    height: Val::Px(8.0),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(crate::palette::EDGE),
            ))
            .with_children(|t| {
                t.spawn((
                    ScrubFill,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        width: Val::Percent(0.0),
                        ..default()
                    },
                    BackgroundColor(crate::palette::ACC),
                ));
                t.spawn((
                    Playhead,
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        left: Val::Percent(0.0),
                        width: Val::Px(3.0),
                        ..default()
                    },
                    BackgroundColor(crate::palette::INK),
                ));
            });

            // step / time readout
            p.spawn((
                TransportLabel,
                Text::new(""),
                TextFont {
                    font_size: 12.0_f32.into(),
                    ..default()
                },
                TextColor(crate::palette::MUT),
                Node {
                    min_width: Val::Px(150.0),
                    ..default()
                },
            ));
        });
}

/// Show the bar only while a plan with steps is loaded.
pub fn transport_visibility(plan: Res<Plan>, mut bar: Query<&mut Visibility, With<TransportBar>>) {
    let Ok(mut vis) = bar.single_mut() else {
        return;
    };
    let want = if plan.steps.is_empty() {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    if *vis != want {
        *vis = want;
    }
}

/// Rebuild the per-step notches when the plan length changes.
pub fn rebuild_notches(
    mut commands: Commands,
    plan: Res<Plan>,
    mut state: ResMut<Transport>,
    track: Query<Entity, With<ScrubTrack>>,
    notches: Query<Entity, With<StepNotch>>,
) {
    let n = plan.steps.len();
    if n == state.built_for {
        return;
    }
    state.built_for = n;
    for e in &notches {
        commands.entity(e).despawn();
    }
    let Ok(track) = track.single() else {
        return;
    };
    if !(2..=MAX_NOTCHES).contains(&n) {
        return;
    }
    commands.entity(track).with_children(|t| {
        for i in 1..n {
            t.spawn((
                StepNotch,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    left: Val::Percent(100.0 * i as f32 / n as f32),
                    width: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(crate::palette::BG2),
            ));
        }
    });
}

/// Drive the fill width, playhead position, play icon and readout from `Plan`.
pub fn transport_sync(
    plan: Res<Plan>,
    mut fill: Query<&mut Node, (With<ScrubFill>, Without<Playhead>)>,
    mut head: Query<&mut Node, (With<Playhead>, Without<ScrubFill>)>,
    mut icon: Query<&mut Text, (With<PlayIcon>, Without<TransportLabel>)>,
    mut label: Query<&mut Text, (With<TransportLabel>, Without<PlayIcon>)>,
) {
    let n = plan.steps.len().max(1) as f32;
    let frac = (plan.t / n).clamp(0.0, 1.0) * 100.0;
    if let Ok(mut f) = fill.single_mut() {
        f.width = Val::Percent(frac);
    }
    if let Ok(mut h) = head.single_mut() {
        h.left = Val::Percent(frac);
    }
    if let Ok(mut t) = icon.single_mut() {
        let glyph = if plan.playing { "\u{23F8}" } else { "\u{25B6}" }; // ⏸ / ▶
        if t.0 != glyph {
            *t = Text::new(glyph);
        }
    }
    if let Ok(mut l) = label.single_mut() {
        *l = Text::new(readout(&plan));
    }
}

/// `step k/n · <action>` plus the temporal time/duration when the plan carries it.
fn readout(plan: &Plan) -> String {
    let n = plan.steps.len();
    if n == 0 {
        return String::new();
    }
    let k = (plan.t.floor() as usize).min(n - 1);
    if plan.t as usize >= n {
        return format!("done · {n} steps");
    }
    let step = &plan.steps[k];
    let mut s = format!("step {}/{} · {}", k + 1, n, step.action.to_lowercase());
    match (step.time, step.duration) {
        (Some(t), Some(d)) => s.push_str(&format!("  ·  t={t:.2} dur={d:.2}")),
        (Some(t), None) => s.push_str(&format!("  ·  t={t:.2}")),
        _ => {}
    }
    s
}

/// Handle the play button and scrubbing on the track.
pub fn transport_input(
    mouse: Res<ButtonInput<MouseButton>>,
    mut transport: ResMut<Transport>,
    mut plan: ResMut<Plan>,
    play_btn: Query<&Interaction, (With<PlayButton>, Changed<Interaction>)>,
    track: Query<(&Interaction, &RelativeCursorPosition), With<ScrubTrack>>,
) {
    if plan.steps.is_empty() {
        transport.hovering = false;
        return;
    }
    let n = plan.steps.len() as f32;

    // play / pause toggle
    for it in &play_btn {
        if *it == Interaction::Pressed {
            if plan.t >= n {
                plan.t = 0.0;
            }
            plan.playing = !plan.playing;
        }
    }

    // scrub: while the pointer is over the track and the button is held, map the
    // normalized x to a timeline position and pause.
    let mut hovering = false;
    if let Ok((_, rel)) = track.single() {
        hovering = rel.cursor_over();
        if hovering && mouse.pressed(MouseButton::Left) {
            if let Some(p) = rel.normalized {
                plan.t = (p.x.clamp(0.0, 1.0) * n).min(n);
                plan.playing = false;
            }
        }
    }
    transport.hovering = hovering;
}
