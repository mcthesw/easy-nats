use std::time::Duration;

use eframe::egui;
use egui::RichText;
use nats_backend::{
    BackendCommand, BackendHandle, ClientConnectionState, ClientStatusQuery, ClientStatusRow,
    ClientStatusSort,
};

use crate::i18n::t;

use super::common::{auto_refresh_ui, format_bytes};
use super::types::ClientStatusState;

pub(crate) fn clients_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut ClientStatusState,
    backend: &BackendHandle,
) {
    ui.heading(t("common.tab_clients"));
    ui.add_space(6.0);

    if !state.endpoint_configured() {
        render_empty_state(ui, t("metrics.no_endpoint"));
        return;
    }

    maybe_start_initial_client_refresh(connection_id, state, backend);
    render_client_controls(ui, connection_id, state, backend);
    handle_client_auto_refresh(ui, connection_id, state, backend);
    ui.add_space(10.0);

    if let Some(error) = state.error() {
        let text = if state.is_stale() {
            t("metrics.client_error_stale")
        } else {
            t("metrics.client_error_unavailable")
        };
        ui.label(
            RichText::new(format!("{text}: {}", error.message))
                .color(ui.visuals().warn_fg_color)
                .strong(),
        );
        ui.add_space(6.0);
    }

    if state.loading && state.page().is_none() {
        render_loading_state(ui, t("metrics.client_loading"));
        return;
    }

    if let Some(page) = state.page() {
        let page_total = page.total;
        let page_offset = page.offset;
        let page_limit = page.limit;
        let rows = page.clients.clone();

        ui.horizontal_wrapped(|ui| {
            ui.weak(format!(
                "{} {}-{} / {}",
                t("metrics.client_page_summary"),
                page_offset.saturating_add(1),
                page_offset.saturating_add(rows.len()),
                page_total
            ));
        });
        ui.add_space(6.0);

        let selected = render_client_table(ui, state.selected_client_id(), &rows);
        if let Some(client_id) = selected {
            start_client_detail_refresh(connection_id, state, backend, client_id);
        }

        ui.add_space(12.0);
        render_client_detail(ui, state);

        if page_total == 0 || page_limit == 0 || rows.is_empty() && !state.loading {
            render_empty_state(ui, t("metrics.client_empty"));
        }
    } else if !state.loading {
        render_empty_state(ui, t("metrics.client_not_loaded"));
    }
}

fn render_client_controls(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut ClientStatusState,
    backend: &BackendHandle,
) {
    let mut trigger_refresh = false;
    ui.horizontal_wrapped(|ui| {
        auto_refresh_ui(ui, "clients_auto_refresh", &mut state.auto_refresh);

        let mut selected_state = state.query().state;
        egui::ComboBox::from_id_salt("clients_state")
            .width(90.0)
            .selected_text(client_state_label(selected_state))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut selected_state,
                    ClientConnectionState::Open,
                    client_state_label(ClientConnectionState::Open),
                );
                ui.selectable_value(
                    &mut selected_state,
                    ClientConnectionState::Closed,
                    client_state_label(ClientConnectionState::Closed),
                );
                ui.selectable_value(
                    &mut selected_state,
                    ClientConnectionState::Any,
                    client_state_label(ClientConnectionState::Any),
                );
            });
        if selected_state != state.query().state {
            state.set_state(selected_state);
            trigger_refresh = true;
        }

        let mut selected_sort = state.query().sort;
        egui::ComboBox::from_id_salt("clients_sort")
            .width(120.0)
            .selected_text(client_sort_label(selected_sort))
            .show_ui(ui, |ui| {
                for sort in client_sort_options(state.query().state) {
                    ui.selectable_value(&mut selected_sort, sort, client_sort_label(sort));
                }
            });
        if selected_sort != state.query().sort {
            state.set_sort(selected_sort);
            trigger_refresh = true;
        }

        let mut selected_page_size = state.query().page_size;
        egui::ComboBox::from_id_salt("clients_page_size")
            .width(72.0)
            .selected_text(selected_page_size.to_string())
            .show_ui(ui, |ui| {
                for page_size in ClientStatusQuery::PAGE_SIZE_OPTIONS {
                    ui.selectable_value(&mut selected_page_size, page_size, page_size.to_string());
                }
            });
        if selected_page_size != state.query().page_size {
            state.set_page_size(selected_page_size);
            trigger_refresh = true;
        }

        if ui
            .add_enabled(!state.loading, egui::Button::new("↻"))
            .on_hover_text(t("metrics.client_refresh"))
            .clicked()
        {
            trigger_refresh = true;
        }

        let has_previous = state.query().offset > 0;
        if ui
            .add_enabled(has_previous && !state.loading, egui::Button::new("←"))
            .on_hover_text(t("metrics.client_previous"))
            .clicked()
        {
            state.previous_page();
            trigger_refresh = true;
        }

        let has_next = state
            .page()
            .is_some_and(|page| page.offset.saturating_add(page.limit) < page.total as usize);
        if ui
            .add_enabled(has_next && !state.loading, egui::Button::new("→"))
            .on_hover_text(t("metrics.client_next"))
            .clicked()
        {
            state.next_page();
            trigger_refresh = true;
        }

        if state.loading {
            ui.spinner();
        }
    });

    if trigger_refresh {
        start_client_page_refresh(connection_id, state, backend);
    }
}

fn maybe_start_initial_client_refresh(
    connection_id: u64,
    state: &mut ClientStatusState,
    backend: &BackendHandle,
) {
    if state.page().is_none()
        && state.error().is_none()
        && !state.loading
        && state.endpoint_configured()
    {
        start_client_page_refresh(connection_id, state, backend);
    }
}

fn handle_client_auto_refresh(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut ClientStatusState,
    backend: &BackendHandle,
) {
    if state.should_refresh() {
        start_client_page_refresh(connection_id, state, backend);
    }

    if state.auto_refresh.enabled {
        ui.ctx().request_repaint_after(Duration::from_secs(1));
    }
}

fn start_client_page_refresh(
    connection_id: u64,
    state: &mut ClientStatusState,
    backend: &BackendHandle,
) {
    state.begin_page_refresh();
    backend.send(BackendCommand::FetchClientStatusPage {
        connection_id,
        endpoint: state.endpoint().to_string(),
        query: state.query().clone(),
    });
}

fn start_client_detail_refresh(
    connection_id: u64,
    state: &mut ClientStatusState,
    backend: &BackendHandle,
    client_id: u64,
) {
    state.begin_detail_refresh(client_id);
    backend.send(BackendCommand::FetchClientStatusDetail {
        connection_id,
        endpoint: state.endpoint().to_string(),
        query: state.detail_query(client_id),
    });
}

fn render_client_table(
    ui: &mut egui::Ui,
    selected_client_id: Option<u64>,
    rows: &[ClientStatusRow],
) -> Option<u64> {
    let mut selected = None;
    egui::ScrollArea::both()
        .id_salt("metrics_client_table_scroll")
        .auto_shrink([false, false])
        .max_height(260.0)
        .show(ui, |ui| {
            render_table_header(ui);
            for (row_index, row) in rows.iter().enumerate() {
                if render_client_row(ui, row_index, selected_client_id, row) {
                    selected = Some(row.client_id);
                }
            }
        });
    selected
}

fn render_table_header(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = CLIENT_TABLE_SPACING;
        for column in CLIENT_TABLE_COLUMNS {
            ui.add_sized(
                [column.width, CLIENT_TABLE_ROW_HEIGHT],
                egui::Label::new(RichText::new(t(column.label_key)).strong()).truncate(),
            )
            .on_hover_text(t(column.label_key));
        }
    });
}

fn render_client_row(
    ui: &mut egui::Ui,
    row_index: usize,
    selected_client_id: Option<u64>,
    row: &ClientStatusRow,
) -> bool {
    let selected = selected_client_id == Some(row.client_id);
    let cells = [
        row.client_id.to_string(),
        client_state_label(row.state).to_string(),
        identity_label(row),
        row.remote_address().unwrap_or_else(missing_label),
        format!(
            "{} / {}",
            row.uptime.as_deref().unwrap_or(t("metrics.client_missing")),
            row.idle.as_deref().unwrap_or(t("metrics.client_missing"))
        ),
        format_opt_count(row.subscriptions),
        row.pending_bytes
            .map(format_bytes)
            .unwrap_or_else(missing_label),
        format!(
            "{} / {}",
            format_opt_count(row.in_msgs),
            format_opt_count(row.out_msgs)
        ),
        format!(
            "{} / {}",
            row.in_bytes.map(format_bytes).unwrap_or_else(missing_label),
            row.out_bytes
                .map(format_bytes)
                .unwrap_or_else(missing_label)
        ),
        row.last_activity
            .as_deref()
            .unwrap_or(t("metrics.client_missing"))
            .to_string(),
        row.rtt
            .as_deref()
            .unwrap_or(t("metrics.client_missing"))
            .to_string(),
        row.closed_reason
            .as_deref()
            .unwrap_or(t("metrics.client_missing"))
            .to_string(),
    ];

    let row_size = egui::vec2(client_table_width(), CLIENT_TABLE_ROW_HEIGHT);
    let (row_rect, _) = ui.allocate_exact_size(row_size, egui::Sense::hover());
    let hovered = ui.rect_contains_pointer(row_rect);
    if let Some(fill) = client_row_fill(ui, selected, hovered, row_index) {
        ui.painter().rect_filled(row_rect, 2.0, fill);
    }

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(row_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = CLIENT_TABLE_SPACING;
            let text_color = if selected {
                Some(ui.visuals().selection.stroke.color)
            } else {
                None
            };
            for (cell, column) in cells.into_iter().zip(CLIENT_TABLE_COLUMNS) {
                let mut text = RichText::new(cell);
                if column.monospace {
                    text = text.monospace();
                }
                if let Some(color) = text_color {
                    text = text.color(color);
                }
                ui.add_sized(
                    [column.width, CLIENT_TABLE_ROW_HEIGHT],
                    egui::Label::new(text).truncate(),
                );
            }
        },
    );

    ui.interact(
        row_rect,
        ui.id().with(("client_row", row.client_id, row_index)),
        egui::Sense::click(),
    )
    .on_hover_text(t("metrics.client_select_hint"))
    .clicked()
}

#[derive(Clone, Copy)]
struct ClientTableColumn {
    label_key: &'static str,
    width: f32,
    monospace: bool,
}

const CLIENT_TABLE_ROW_HEIGHT: f32 = 24.0;
const CLIENT_TABLE_SPACING: f32 = 14.0;
const CLIENT_TABLE_COLUMNS: &[ClientTableColumn] = &[
    ClientTableColumn {
        label_key: "metrics.client_col_client",
        width: 72.0,
        monospace: true,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_state",
        width: 90.0,
        monospace: false,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_identity",
        width: 210.0,
        monospace: false,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_address",
        width: 170.0,
        monospace: true,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_uptime_idle",
        width: 120.0,
        monospace: false,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_subs",
        width: 72.0,
        monospace: true,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_pending",
        width: 110.0,
        monospace: false,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_msgs",
        width: 125.0,
        monospace: true,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_bytes",
        width: 135.0,
        monospace: false,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_activity",
        width: 230.0,
        monospace: true,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_rtt",
        width: 90.0,
        monospace: true,
    },
    ClientTableColumn {
        label_key: "metrics.client_col_reason",
        width: 140.0,
        monospace: false,
    },
];

fn client_table_width() -> f32 {
    let column_width: f32 = CLIENT_TABLE_COLUMNS.iter().map(|column| column.width).sum();
    let spacing = CLIENT_TABLE_SPACING * CLIENT_TABLE_COLUMNS.len().saturating_sub(1) as f32;
    column_width + spacing
}

fn client_row_fill(
    ui: &egui::Ui,
    selected: bool,
    hovered: bool,
    row_index: usize,
) -> Option<egui::Color32> {
    if selected {
        return Some(ui.visuals().selection.bg_fill.gamma_multiply(0.65));
    }

    if hovered {
        return Some(ui.visuals().widgets.hovered.bg_fill.gamma_multiply(0.35));
    }

    if row_index % 2 == 1 {
        return Some(ui.visuals().faint_bg_color);
    }

    None
}

fn render_client_detail(ui: &mut egui::Ui, state: &mut ClientStatusState) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new(t("metrics.client_detail")).strong());
        if state.detail_loading {
            ui.spinner();
        }
        if state.selected_client_id().is_some() && ui.small_button(t("common.clear")).clicked() {
            state.clear_selected_client();
        }
    });
    ui.add_space(4.0);

    if state.detail_loading && state.detail().is_none() {
        render_loading_state(ui, t("metrics.client_detail_loading"));
        return;
    }

    let Some(detail) = state.detail() else {
        ui.weak(t("metrics.client_select_hint"));
        return;
    };

    if state.selected_detail_stale() {
        ui.label(RichText::new(t("metrics.client_detail_stale")).color(ui.visuals().warn_fg_color));
        ui.add_space(4.0);
    }

    let client = &detail.client;
    egui::Grid::new("metrics_client_detail_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            detail_row(
                ui,
                t("metrics.client_col_client"),
                &client.client_id.to_string(),
            );
            detail_row(
                ui,
                t("metrics.client_col_state"),
                client_state_label(client.state),
            );
            detail_row(
                ui,
                t("metrics.client_col_identity"),
                &identity_label(client),
            );
            detail_row(
                ui,
                t("metrics.client_col_address"),
                &client.remote_address().unwrap_or_else(missing_label),
            );
            detail_row(
                ui,
                t("metrics.client_col_uptime_idle"),
                &format!(
                    "{} / {}",
                    client
                        .uptime
                        .as_deref()
                        .unwrap_or(t("metrics.client_missing")),
                    client
                        .idle
                        .as_deref()
                        .unwrap_or(t("metrics.client_missing"))
                ),
            );
            detail_row(
                ui,
                t("metrics.client_col_pending"),
                &format_opt_count(client.pending_bytes),
            );
            detail_row(
                ui,
                t("metrics.client_col_activity"),
                client
                    .last_activity
                    .as_deref()
                    .unwrap_or(t("metrics.client_missing")),
            );
            detail_row(
                ui,
                t("metrics.client_col_rtt"),
                client.rtt.as_deref().unwrap_or(t("metrics.client_missing")),
            );
        });

    ui.add_space(8.0);
    ui.label(RichText::new(t("metrics.client_subscriptions")).strong());
    if client.subscription_details.is_empty() {
        ui.weak(t("metrics.client_no_subscriptions"));
    } else {
        egui::ScrollArea::vertical()
            .id_salt("metrics_client_subscriptions")
            .max_height(120.0)
            .show(ui, |ui| {
                for subscription in &client.subscription_details {
                    ui.monospace(&subscription.subject);
                }
            });
    }
}

fn detail_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.weak(label);
    ui.label(value);
    ui.end_row();
}

fn identity_label(row: &ClientStatusRow) -> String {
    let mut parts = Vec::new();
    if let Some(name) = &row.name {
        parts.push(name.clone());
    }
    if let Some(user) = &row.user {
        parts.push(user.clone());
    }
    if let Some(account) = &row.account {
        parts.push(format!("@{account}"));
    }
    if parts.is_empty() {
        missing_label()
    } else {
        parts.join(" ")
    }
}

fn client_state_label(state: ClientConnectionState) -> &'static str {
    match state {
        ClientConnectionState::Open => t("metrics.client_state_open"),
        ClientConnectionState::Closed => t("metrics.client_state_closed"),
        ClientConnectionState::Any => t("metrics.client_state_any"),
    }
}

fn client_sort_label(sort: ClientStatusSort) -> &'static str {
    match sort {
        ClientStatusSort::Cid => t("metrics.client_sort_cid"),
        ClientStatusSort::Start => t("metrics.client_sort_start"),
        ClientStatusSort::Subscriptions => t("metrics.client_sort_subs"),
        ClientStatusSort::PendingBytes => t("metrics.client_sort_pending"),
        ClientStatusSort::InMessages => t("metrics.client_sort_in_msgs"),
        ClientStatusSort::OutMessages => t("metrics.client_sort_out_msgs"),
        ClientStatusSort::InBytes => t("metrics.client_sort_in_bytes"),
        ClientStatusSort::OutBytes => t("metrics.client_sort_out_bytes"),
        ClientStatusSort::LastActivity => t("metrics.client_sort_last"),
        ClientStatusSort::Idle => t("metrics.client_sort_idle"),
        ClientStatusSort::Uptime => t("metrics.client_sort_uptime"),
        ClientStatusSort::Stop => t("metrics.client_sort_stop"),
        ClientStatusSort::Reason => t("metrics.client_sort_reason"),
    }
}

fn client_sort_options(state: ClientConnectionState) -> impl Iterator<Item = ClientStatusSort> {
    ClientStatusSort::ALL
        .into_iter()
        .filter(move |sort| sort.is_allowed_for_state(state))
}

fn format_opt_count(value: Option<u64>) -> String {
    value.map_or_else(missing_label, |value| value.to_string())
}

fn missing_label() -> String {
    t("metrics.client_missing").to_string()
}

fn render_loading_state(ui: &mut egui::Ui, message: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(16.0);
        ui.spinner();
        ui.add_space(6.0);
        ui.label(message);
    });
}

fn render_empty_state(ui: &mut egui::Ui, message: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(16.0);
        ui.weak(message);
    });
}
