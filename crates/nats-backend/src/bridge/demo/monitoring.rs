use std::cmp::Ordering;

use crate::event::BackendEvent;
use crate::monitoring::{
    ClientConnectionState, ClientStatusDetail, ClientStatusPage, ClientStatusQuery,
    ClientStatusRow, ClientStatusSort,
};

use super::{DemoState, fixtures};

impl DemoState {
    pub(super) fn get_server_info(&mut self, connection_id: u64) {
        self.push(BackendEvent::ServerInfoLoaded {
            connection_id,
            info: fixtures::server_info(),
        });
    }

    pub(super) fn get_jetstream_account_info(&mut self, connection_id: u64) {
        self.push(BackendEvent::JetStreamAccountInfoLoaded {
            connection_id,
            info: fixtures::account_info(),
        });
    }

    pub(super) fn fetch_metrics(&mut self, connection_id: u64, endpoint: String) {
        self.push(BackendEvent::MetricsSnapshot {
            connection_id,
            snapshot: Box::new(fixtures::metrics(endpoint, self.synthetic_count)),
        });
    }

    pub(super) fn fetch_client_status_page(
        &mut self,
        connection_id: u64,
        endpoint: String,
        query: ClientStatusQuery,
    ) {
        let mut clients: Vec<_> = fixtures::clients()
            .into_iter()
            .filter(|client| match query.state {
                ClientConnectionState::Open => client.state == ClientConnectionState::Open,
                ClientConnectionState::Closed => client.state == ClientConnectionState::Closed,
                ClientConnectionState::Any => true,
            })
            .collect();
        clients.sort_by(|left, right| {
            compare_clients(left, right, query.sort)
                .then_with(|| left.client_id.cmp(&right.client_id))
        });
        let total = clients.len() as u64;
        let page_clients = clients
            .into_iter()
            .skip(query.offset)
            .take(query.page_size)
            .collect();
        self.push(BackendEvent::ClientStatusPageLoaded {
            connection_id,
            page: Box::new(ClientStatusPage {
                endpoint,
                collected_at: fixtures::system_time(),
                offset: query.offset,
                limit: query.page_size,
                query,
                total,
                clients: page_clients,
            }),
        });
    }

    pub(super) fn fetch_client_status_detail(
        &mut self,
        connection_id: u64,
        endpoint: String,
        query: ClientStatusQuery,
    ) {
        let client = fixtures::clients()
            .into_iter()
            .find(|client| Some(client.client_id) == query.client_id)
            .unwrap_or_else(|| fixtures::clients().remove(0));
        self.push(BackendEvent::ClientStatusDetailLoaded {
            connection_id,
            detail: Box::new(ClientStatusDetail {
                endpoint,
                collected_at: fixtures::system_time(),
                query,
                client,
            }),
        });
    }
}

fn compare_clients(
    left: &ClientStatusRow,
    right: &ClientStatusRow,
    sort: ClientStatusSort,
) -> Ordering {
    match sort {
        ClientStatusSort::Cid => left.client_id.cmp(&right.client_id),
        ClientStatusSort::Start | ClientStatusSort::Uptime => left.uptime.cmp(&right.uptime),
        ClientStatusSort::Subscriptions => left.subscriptions.cmp(&right.subscriptions),
        ClientStatusSort::PendingBytes => left.pending_bytes.cmp(&right.pending_bytes),
        ClientStatusSort::InMessages => left.in_msgs.cmp(&right.in_msgs),
        ClientStatusSort::OutMessages => left.out_msgs.cmp(&right.out_msgs),
        ClientStatusSort::InBytes => left.in_bytes.cmp(&right.in_bytes),
        ClientStatusSort::OutBytes => left.out_bytes.cmp(&right.out_bytes),
        ClientStatusSort::LastActivity => left.last_activity.cmp(&right.last_activity),
        ClientStatusSort::Idle => left.idle.cmp(&right.idle),
        ClientStatusSort::Stop => left.closed_at.cmp(&right.closed_at),
        ClientStatusSort::Reason => left.closed_reason.cmp(&right.closed_reason),
    }
}
