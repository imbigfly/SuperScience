use crate::app_support::compose_icon;
use crate::chat_render::fmt_tokens;
use crate::dto::{TrajectoryCellDto, TrajectorySnapshotDto, TrajectoryStatsDto, TrajectoryUsageDto};
use crate::i18n::{t, tf, use_locale, Locale};
use crate::text::{event_target_value, format_duration_ms};
use leptos::*;
use std::collections::HashSet;

/// Which pane the thread area shows: the ordinary chat transcript or the
/// per-turn event trajectory (轨迹).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ThreadView {
    #[default]
    Chat,
    Trajectory,
}

/// Per-turn split of wall time: idle/input gap, model time, tool time.
/// Derived from cell timestamps and durations; `None` when the cells carry
/// no usable timing at all.
fn turn_timing(cells: &[TrajectoryCellDto]) -> Option<(u64, u64, u64)> {
    let mut model_ms = 0i64;
    let mut tool_ms = 0i64;
    let mut start: Option<i64> = None;
    let mut end: Option<i64> = None;
    for cell in cells {
        match cell.kind.as_str() {
            "assistant" => model_ms += cell.duration_ms.unwrap_or(0),
            "tool" => tool_ms += cell.duration_ms.unwrap_or(0),
            _ => {}
        }
        if let Some(ts) = cell.ts {
            let cell_end = ts + cell.duration_ms.unwrap_or(0);
            start = Some(start.map_or(ts, |s| s.min(ts)));
            end = Some(end.map_or(cell_end, |e| e.max(cell_end)));
        }
    }
    let span = end? - start?;
    if span <= 0 {
        return None;
    }
    let input_ms = (span - model_ms - tool_ms).max(0) as u64;
    Some((input_ms, model_ms.max(0) as u64, tool_ms.max(0) as u64))
}

fn badge_label(kind: &str) -> String {
    match kind {
        "user" => "USER".to_string(),
        "assistant" => "ASSISTANT".to_string(),
        "tool" => "TOOL".to_string(),
        "usage" => "USAGE".to_string(),
        other => other.to_uppercase(),
    }
}

fn cell_matches(cell: &TrajectoryCellDto, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let hit = |text: &str| text.to_lowercase().contains(query);
    hit(&cell.summary)
        || cell.detail_input.as_deref().is_some_and(hit)
        || cell.detail_output.as_deref().is_some_and(hit)
}

fn usage_line(locale: Locale, usage: &TrajectoryUsageDto) -> String {
    let mut line = tf(
        locale,
        "trajectory.usage.line",
        &[
            ("round", &usage.round.to_string()),
            ("in", &fmt_tokens(usage.input_tokens.max(0) as u64)),
            ("out", &fmt_tokens(usage.output_tokens.max(0) as u64)),
        ],
    );
    if usage.input_tokens > 0 && usage.cached_input_tokens > 0 {
        let pct = (usage.cached_input_tokens as f64 * 100.0 / usage.input_tokens as f64).round();
        line.push_str(&tf(
            locale,
            "trajectory.usage.cached",
            &[("pct", &format!("{pct:.0}"))],
        ));
    }
    line
}

fn stats_line(locale: Locale, stats: &TrajectoryStatsDto) -> String {
    let dash = "–".to_string();
    let tok_per_sec = stats
        .tokens_per_sec
        .map(|v| format!("{v:.1}"))
        .unwrap_or_else(|| dash.clone());
    let cache_pct = stats
        .cache_hit_pct
        .map(|v| format!("{v:.0}"))
        .unwrap_or(dash);
    format!(
        "{} · {} | LLM {} · {} {} | {} tok/s | {} | {} · {}",
        tf(
            locale,
            "trajectory.stats.turns",
            &[("n", &stats.turns.to_string())]
        ),
        tf(
            locale,
            "trajectory.stats.steps",
            &[("n", &stats.steps.to_string())]
        ),
        format_duration_ms(stats.llm_ms.max(0) as u64),
        t(locale, "trajectory.legend.tools"),
        format_duration_ms(stats.tool_ms.max(0) as u64),
        tok_per_sec,
        tf(locale, "trajectory.stats.cache_hit", &[("pct", &cache_pct)]),
        tf(
            locale,
            "trajectory.stats.input",
            &[("tokens", &fmt_tokens(stats.input_tokens.max(0) as u64))]
        ),
        tf(
            locale,
            "trajectory.stats.output",
            &[("tokens", &fmt_tokens(stats.output_tokens.max(0) as u64))]
        ),
    )
}

#[component]
fn TrajectoryCellRow(
    cell: TrajectoryCellDto,
    cell_key: String,
    expanded: RwSignal<HashSet<String>>,
) -> impl IntoView {
    let locale = use_locale();
    let is_usage = cell.kind == "usage";
    let detail_input = cell.detail_input.clone();
    let detail_output = cell.detail_output.clone();
    let expandable = !is_usage && (detail_input.is_some() || detail_output.is_some());
    let duration = cell.duration_ms.filter(|ms| *ms > 0);
    let row_class = format!(
        "traj-row {}{}",
        cell.kind,
        if expandable { " expandable" } else { "" }
    );
    let toggle_key = cell_key.clone();
    let on_click = move |_| {
        if expandable {
            expanded.update(|set| {
                if !set.remove(&toggle_key) {
                    set.insert(toggle_key.clone());
                }
            });
        }
    };
    view! {
        <div class=row_class
            class:error=cell.is_error
            class:pending=cell.ok.is_none() && cell.kind == "tool"
            data-testid=format!("traj-row-{}", cell.kind)
            on:click=on_click>
            <span class=format!("traj-badge {}", cell.kind)>{badge_label(&cell.kind)}</span>
            <span class="traj-summary">{move || {
                if is_usage {
                    cell.usage
                        .as_ref()
                        .map(|usage| usage_line(locale.get(), usage))
                        .unwrap_or_default()
                } else {
                    cell.summary.clone()
                }
            }}</span>
            {duration.map(|ms| view! {
                <span class="traj-duration">{format_duration_ms(ms.max(0) as u64)}</span>
            })}
        </div>
        {move || {
            let open = expanded.with(|set| set.contains(&cell_key));
            (open && expandable).then(|| view! {
                <div class="traj-detail" data-testid="traj-detail">
                    {detail_input.as_ref().map(|input| view! {
                        <span class="traj-detail-label">{t(locale.get(), "trajectory.detail_input")}</span>
                        <pre data-testid="traj-detail-input">{input.clone()}</pre>
                    })}
                    {detail_output.as_ref().map(|output| view! {
                        <span class="traj-detail-label">{t(locale.get(), "trajectory.detail_output")}</span>
                        <pre data-testid="traj-detail-output">{output.clone()}</pre>
                    })}
                </div>
            })
        }}
    }
}

#[component]
pub(crate) fn TrajectoryView(
    snapshot: RwSignal<Option<TrajectorySnapshotDto>>,
    live: RwSignal<Vec<TrajectoryCellDto>>,
    busy: RwSignal<bool>,
) -> impl IntoView {
    let locale = use_locale();
    let query = create_rw_signal(String::new());
    let expanded = create_rw_signal(HashSet::<String>::new());

    view! {
        <div class="trajectory" data-testid="trajectory-view">
            <div class="trajectory-toolbar">
                <div class="trajectory-search">
                    {compose_icon("search")}
                    <input type="search"
                        placeholder=move || t(locale.get(), "trajectory.search")
                        aria-label=move || t(locale.get(), "trajectory.search")
                        prop:value=query
                        on:input=move |ev| query.set(event_target_value(&ev)) />
                </div>
                <div class="trajectory-legend">
                    <span><i class="traj-swatch input"></i>{move || t(locale.get(), "trajectory.legend.input")}</span>
                    <span><i class="traj-swatch model"></i>{move || t(locale.get(), "trajectory.legend.model")}</span>
                    <span><i class="traj-swatch tools"></i>{move || t(locale.get(), "trajectory.legend.tools")}</span>
                </div>
            </div>
            {move || {
                let loc = locale.get();
                let q = query.get().trim().to_lowercase();
                let snap = snapshot.get();
                let live_cells = live.get();
                let running = busy.get();
                let turns = snap.as_ref().map(|s| s.turns.len()).unwrap_or(0);
                if turns == 0 && live_cells.is_empty() {
                    return view! {
                        <div class="trajectory-empty">{t(loc, "trajectory.empty")}</div>
                    }.into_view();
                }
                let mut any_visible = false;
                let turn_views = snap
                    .as_ref()
                    .map(|s| {
                        s.turns
                            .iter()
                            .filter_map(|turn| {
                                let cells: Vec<(usize, &TrajectoryCellDto)> = turn
                                    .cells
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, cell)| cell_matches(cell, &q))
                                    .collect();
                                if cells.is_empty() {
                                    return None;
                                }
                                any_visible = true;
                                let timing = turn_timing(&turn.cells);
                                let running_turn = running && turn.index as usize == turns;
                                Some(view! {
                                    <section class="traj-turn">
                                        <div class="traj-turn-head">
                                            <span>{tf(loc, "trajectory.turn", &[("n", &turn.index.to_string())])}</span>
                                            {running_turn.then(|| view! {
                                                <span class="traj-running">{t(loc, "trajectory.running")}</span>
                                            })}
                                        </div>
                                        {timing.map(|(input_ms, model_ms, tool_ms)| view! {
                                            <div class="traj-bar">
                                                {(input_ms > 0).then(|| view! {
                                                    <div class="traj-bar-seg input" style=format!("flex-grow:{input_ms}")></div>
                                                })}
                                                {(model_ms > 0).then(|| view! {
                                                    <div class="traj-bar-seg model" style=format!("flex-grow:{model_ms}")></div>
                                                })}
                                                {(tool_ms > 0).then(|| view! {
                                                    <div class="traj-bar-seg tools" style=format!("flex-grow:{tool_ms}")></div>
                                                })}
                                            </div>
                                        })}
                                        <div class="traj-rows">
                                            {cells.into_iter().map(|(ci, cell)| {
                                                let key = format!("{}:{ci}", turn.index);
                                                view! {
                                                    <TrajectoryCellRow
                                                        cell=cell.clone()
                                                        cell_key=key
                                                        expanded=expanded />
                                                }
                                            }).collect_view()}
                                        </div>
                                    </section>
                                })
                            })
                            .collect_view()
                    })
                    .unwrap_or_default();
                // Live cells of the in-flight turn trail the persisted turns;
                // the Done refetch replaces them with exact backend data.
                let live_view = (!live_cells.is_empty()).then(|| {
                    let next_turn = turns as i64 + 1;
                    let visible: Vec<(usize, &TrajectoryCellDto)> = live_cells
                        .iter()
                        .enumerate()
                        .filter(|(_, cell)| cell_matches(cell, &q))
                        .collect();
                    view! {
                        <section class="traj-turn live" data-testid="traj-live-turn">
                            <div class="traj-turn-head">
                                <span>{tf(loc, "trajectory.turn", &[("n", &next_turn.to_string())])}</span>
                                <span class="traj-running">{t(loc, "trajectory.running")}</span>
                            </div>
                            <div class="traj-rows">
                                {visible.into_iter().map(|(ci, cell)| {
                                    let key = format!("live:{ci}");
                                    view! {
                                        <TrajectoryCellRow
                                            cell=cell.clone()
                                            cell_key=key
                                            expanded=expanded />
                                    }
                                }).collect_view()}
                            </div>
                        </section>
                    }
                });
                let footer = snap.as_ref().map(|s| {
                    view! {
                        <div class="trajectory-footer" data-testid="trajectory-footer">
                            {stats_line(loc, &s.stats)}
                        </div>
                    }
                });
                let no_match = (!any_visible && !q.is_empty() && live_cells.is_empty()).then(|| {
                    view! { <div class="trajectory-empty">{t(loc, "trajectory.no_match")}</div> }
                });
                view! {
                    {turn_views}
                    {live_view}
                    {no_match}
                    {footer}
                }.into_view()
            }}
        </div>
    }
}
