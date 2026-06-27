use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eframe::egui;
use egui::RichText;
use egui_plot::{Corner, HoverPosition, Legend, Line, Plot, PlotPoints};
use nats_backend::{BackendCommand, BackendHandle, MetricsSection, MetricsSnapshot, VarzMetrics};

use crate::i18n::t;

use super::common::{auto_refresh_ui, format_bytes};
use super::types::MetricsState;

pub fn metrics_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut MetricsState,
    backend: &BackendHandle,
) {
    maybe_start_initial_refresh(connection_id, state, backend);

    ui.horizontal(|ui| {
        ui.heading(t("metrics.title"));
        let can_refresh = state.endpoint_configured() && !state.loading;
        if ui
            .add_enabled(can_refresh, egui::Button::new("↻"))
            .on_hover_text(t("metrics.refresh"))
            .clicked()
        {
            start_refresh(connection_id, state, backend);
        }
        if state.loading {
            ui.spinner();
        }
    });
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        auto_refresh_ui(ui, "metrics_auto_refresh", &mut state.auto_refresh);
        if let Some((label, fill)) = status_badge(state, ui.visuals()) {
            ui.add_space(8.0);
            render_status_badge(ui, label, fill);
        }
    });

    handle_auto_refresh(ui, connection_id, state, backend);
    ui.add_space(12.0);

    if !state.endpoint_configured() {
        render_empty_state(ui, t("metrics.no_endpoint"));
        return;
    }

    if let Some(latest_attempt) = state.latest_attempt()
        && !latest_attempt.errors.is_empty()
    {
        render_error_panel(ui, latest_attempt, state.is_stale());
        ui.add_space(8.0);
    }

    if state.loading && state.latest_data().is_none() {
        render_loading_state(ui);
        return;
    }

    let Some(latest) = state.latest_data() else {
        if let Some(latest_attempt) = state.latest_attempt() {
            render_unavailable_state(ui, latest_attempt);
        } else {
            render_empty_state(ui, t("metrics.not_loaded"));
        }
        return;
    };

    render_summary(ui, latest);
    ui.add_space(16.0);
    render_charts(ui, connection_id, state.history());
}

fn maybe_start_initial_refresh(
    connection_id: u64,
    state: &mut MetricsState,
    backend: &BackendHandle,
) {
    if state.endpoint_configured() && state.has_never_loaded() && !state.loading {
        start_refresh(connection_id, state, backend);
    }
}

fn handle_auto_refresh(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut MetricsState,
    backend: &BackendHandle,
) {
    if state.auto_refresh.should_refresh() && state.endpoint_configured() && !state.loading {
        start_refresh(connection_id, state, backend);
    }

    if state.auto_refresh.enabled {
        ui.ctx().request_repaint_after(Duration::from_secs(1));
    }
}

fn start_refresh(connection_id: u64, state: &mut MetricsState, backend: &BackendHandle) {
    state.begin_refresh();
    backend.send(BackendCommand::FetchMetrics {
        connection_id,
        endpoint: state.endpoint().to_string(),
    });
}

fn status_badge(
    state: &MetricsState,
    visuals: &egui::Visuals,
) -> Option<(&'static str, egui::Color32)> {
    if !state.endpoint_configured() {
        return None;
    }

    if state.is_stale() {
        return Some((t("metrics.status_stale"), visuals.warn_fg_color));
    }

    if let Some(snapshot) = state.latest_attempt() {
        if snapshot.is_partial() {
            return Some((t("metrics.status_partial"), visuals.warn_fg_color));
        }
        if let Some(health) = &snapshot.health {
            if !health.ok {
                return Some((t("metrics.status_degraded"), visuals.warn_fg_color));
            }
            return None;
        }
        if state.loading {
            return Some((t("metrics.status_loading"), visuals.widgets.active.bg_fill));
        }
        return Some((t("metrics.status_unavailable"), visuals.error_fg_color));
    }

    if state.loading {
        Some((t("metrics.status_loading"), visuals.widgets.active.bg_fill))
    } else {
        None
    }
}

fn render_status_badge(ui: &mut egui::Ui, label: &str, fill: egui::Color32) {
    egui::Frame::new()
        .fill(fill.gamma_multiply(0.12))
        .corner_radius(999.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.colored_label(fill, label);
        });
}

fn render_loading_state(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.spinner();
        ui.add_space(8.0);
        ui.label(t("metrics.loading"));
    });
}

fn render_empty_state(ui: &mut egui::Ui, message: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.weak(message);
    });
}

fn render_error_panel(ui: &mut egui::Ui, snapshot: &MetricsSnapshot, stale: bool) {
    let warn_color = ui.visuals().warn_fg_color;
    ui.label(
        RichText::new(if stale {
            t("metrics.error_stale")
        } else {
            t("metrics.error_partial")
        })
        .color(warn_color)
        .strong(),
    );
    ui.add_space(2.0);
    for error in &snapshot.errors {
        ui.label(
            RichText::new(format!(
                "{}: {}",
                section_label(error.section),
                error.message
            ))
            .color(warn_color),
        );
    }
}

fn render_unavailable_state(ui: &mut egui::Ui, snapshot: &MetricsSnapshot) {
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.strong(t("metrics.error_unavailable"));
        ui.add_space(6.0);
        for error in &snapshot.errors {
            ui.label(format!(
                "{}: {}",
                section_label(error.section),
                error.message
            ));
        }
    });
}

fn render_summary(ui: &mut egui::Ui, latest: &MetricsSnapshot) {
    let mut cards = Vec::new();
    if let Some(varz) = &latest.varz {
        cards.push((
            t("metrics.card_connections"),
            format_count(varz.connections),
        ));
        cards.push((
            t("metrics.card_subscriptions"),
            format_count(varz.subscriptions),
        ));
        cards.push((t("metrics.card_memory"), format_bytes(varz.mem_bytes)));
        cards.push((t("metrics.card_cpu"), format!("{:.1}%", varz.cpu_percent)));
    }
    if let Some(jsz) = &latest.jsz {
        cards.push((t("metrics.card_js_streams"), format_count(jsz.streams)));
        cards.push((
            t("metrics.card_js_storage"),
            format_bytes(jsz.storage_bytes),
        ));
    }

    if cards.is_empty() {
        ui.weak(t("metrics.summary_empty"));
        return;
    }

    egui::Grid::new("metrics_summary_grid")
        .num_columns(3)
        .spacing([24.0, 16.0])
        .min_col_width(140.0)
        .show(ui, |ui| {
            for row in cards.chunks(3) {
                for (title, value) in row {
                    render_summary_cell(ui, title, value);
                }
                for _ in row.len()..3 {
                    ui.label("");
                }
                ui.end_row();
            }
        });
}

fn render_summary_cell(ui: &mut egui::Ui, title: &str, value: &str) {
    ui.vertical(|ui| {
        ui.set_min_width(0.0);
        ui.label(RichText::new(title).weak());
        ui.add_space(2.0);
        ui.label(RichText::new(value).size(22.0).strong());
    });
}

fn render_charts(
    ui: &mut egui::Ui,
    connection_id: u64,
    history: &std::collections::VecDeque<MetricsSnapshot>,
) {
    if history.len() < 2 {
        ui.weak(t("metrics.charts_need_samples"));
        return;
    }

    let msg_in = rate_series(history, |metrics| metrics.in_msgs);
    let msg_out = rate_series(history, |metrics| metrics.out_msgs);
    let bytes_in = rate_series(history, |metrics| metrics.in_bytes);
    let bytes_out = rate_series(history, |metrics| metrics.out_bytes);
    let js_memory = gauge_series(history, |snapshot| {
        snapshot.jsz.as_ref().map(|jsz| jsz.memory_bytes as f64)
    });
    let js_storage = gauge_series(history, |snapshot| {
        snapshot.jsz.as_ref().map(|jsz| jsz.storage_bytes as f64)
    });

    render_plot(
        ui,
        ("metrics_msg_rate", connection_id),
        t("metrics.chart_msg_rate"),
        t("metrics.chart_msg_rate_y"),
        vec![
            (t("metrics.series_in_msgs"), msg_in),
            (t("metrics.series_out_msgs"), msg_out),
        ],
        ValueFormatter::Count,
    );
    ui.add_space(12.0);
    render_plot(
        ui,
        ("metrics_byte_rate", connection_id),
        t("metrics.chart_byte_rate"),
        t("metrics.chart_byte_rate_y"),
        vec![
            (t("metrics.series_in_bytes"), bytes_in),
            (t("metrics.series_out_bytes"), bytes_out),
        ],
        ValueFormatter::Bytes,
    );
    ui.add_space(12.0);
    render_plot(
        ui,
        ("metrics_js_usage", connection_id),
        t("metrics.chart_js_usage"),
        t("metrics.chart_js_usage_y"),
        vec![
            (t("metrics.series_js_memory"), js_memory),
            (t("metrics.series_js_storage"), js_storage),
        ],
        ValueFormatter::Bytes,
    );
}

fn render_plot(
    ui: &mut egui::Ui,
    id_source: impl egui::AsId,
    title: &str,
    y_axis_label: &str,
    series: Vec<(&str, Vec<[f64; 2]>)>,
    formatter: ValueFormatter,
) {
    let mut reset_view = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            reset_view = ui.small_button(t("metrics.chart_reset")).clicked();
        });
    });
    ui.add_space(4.0);
    let mut plot = Plot::new(id_source)
        .height(180.0)
        .legend(
            Legend::default()
                .position(Corner::RightTop)
                .background_alpha(0.0),
        )
        .show_background(false)
        .allow_boxed_zoom(false)
        .x_axis_formatter(|mark, _range| format_time(mark.value))
        .y_axis_formatter(|mark, _range| formatter.format(mark.value))
        .label_formatter(move |position| {
            let HoverPosition::NearDataPoint {
                plot_name,
                position,
                ..
            } = position
            else {
                return None;
            };
            Some(format!(
                "{plot_name}\n{}\n{}",
                format_time(position.x),
                formatter.format(position.y)
            ))
        })
        .x_axis_label(t("metrics.chart_time"))
        .y_axis_label(y_axis_label);
    if reset_view {
        plot = plot.reset();
    }
    plot.show(ui, |plot_ui| {
        for (name, points) in series {
            if points.is_empty() {
                continue;
            }
            plot_ui.line(Line::new(name, PlotPoints::from(points)));
        }
    });
}

fn rate_series(
    history: &std::collections::VecDeque<MetricsSnapshot>,
    extract: impl Fn(&VarzMetrics) -> u64,
) -> Vec<[f64; 2]> {
    let mut points = Vec::new();
    let mut previous: Option<(&MetricsSnapshot, &VarzMetrics)> = None;

    for snapshot in history {
        let Some(metrics) = snapshot.varz.as_ref() else {
            continue;
        };
        if let Some((prev_snapshot, prev_metrics)) = previous {
            let dt = duration_secs(prev_snapshot.collected_at, snapshot.collected_at);
            if dt > 0.0 {
                let delta = extract(metrics).saturating_sub(extract(prev_metrics)) as f64;
                points.push([timestamp_secs(snapshot.collected_at), delta / dt]);
            }
        }
        previous = Some((snapshot, metrics));
    }

    points
}

fn gauge_series(
    history: &std::collections::VecDeque<MetricsSnapshot>,
    extract: impl Fn(&MetricsSnapshot) -> Option<f64>,
) -> Vec<[f64; 2]> {
    history
        .iter()
        .filter_map(|snapshot| {
            extract(snapshot).map(|value| [timestamp_secs(snapshot.collected_at), value])
        })
        .collect()
}

fn timestamp_secs(time: SystemTime) -> f64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn duration_secs(start: SystemTime, end: SystemTime) -> f64 {
    end.duration_since(start).unwrap_or_default().as_secs_f64()
}

fn format_time(value: f64) -> String {
    let secs = value.max(0.0) as u64;
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn format_count(value: u64) -> String {
    match value {
        0..=999 => value.to_string(),
        1_000..=999_999 => format!("{:.1}k", value as f64 / 1_000.0),
        1_000_000..=999_999_999 => format!("{:.1}M", value as f64 / 1_000_000.0),
        _ => format!("{:.1}B", value as f64 / 1_000_000_000.0),
    }
}

#[derive(Clone, Copy)]
enum ValueFormatter {
    Count,
    Bytes,
}

impl ValueFormatter {
    fn format(self, value: f64) -> String {
        match self {
            Self::Count => {
                if value >= 1000.0 {
                    format_count(value.round().max(0.0) as u64)
                } else {
                    format!("{value:.1}")
                }
            }
            Self::Bytes => format_bytes(value.max(0.0) as u64),
        }
    }
}

fn section_label(section: MetricsSection) -> &'static str {
    match section {
        MetricsSection::Health => t("metrics.section_health"),
        MetricsSection::Varz => t("metrics.section_varz"),
        MetricsSection::Connz => t("metrics.section_connz"),
        MetricsSection::Jsz => t("metrics.section_jsz"),
    }
}
