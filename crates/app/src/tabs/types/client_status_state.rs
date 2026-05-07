use std::time::Instant;

use nats_backend::{
    ClientConnectionState, ClientStatusDetail, ClientStatusPage, ClientStatusQuery,
    ClientStatusRequestError, ClientStatusSort,
};

use super::AutoRefresh;

#[derive(Debug, Default)]
pub struct ClientStatusState {
    endpoint: String,
    query: ClientStatusQuery,
    page: Option<Box<ClientStatusPage>>,
    detail: Option<Box<ClientStatusDetail>>,
    error: Option<Box<ClientStatusRequestError>>,
    selected_client_id: Option<u64>,
    selected_detail_stale: bool,
    pub loading: bool,
    pub detail_loading: bool,
    pub auto_refresh: AutoRefresh,
}

impl ClientStatusState {
    pub fn with_endpoint(endpoint: String) -> Self {
        let mut state = Self {
            endpoint: normalize_monitoring_endpoint(endpoint),
            ..Default::default()
        };
        state.auto_refresh.enabled = true;
        state.auto_refresh.interval_secs = 5;
        state
    }

    pub fn endpoint(&self) -> &str {
        self.endpoint.as_str()
    }

    pub fn endpoint_configured(&self) -> bool {
        !self.endpoint.trim().is_empty()
    }

    pub fn set_endpoint(&mut self, endpoint: String) {
        let endpoint = normalize_monitoring_endpoint(endpoint);
        if self.endpoint == endpoint {
            return;
        }
        self.endpoint = endpoint;
        self.clear_status();
    }

    pub fn query(&self) -> &ClientStatusQuery {
        &self.query
    }

    pub fn detail_query(&self, client_id: u64) -> ClientStatusQuery {
        let mut query = self.query.clone();
        query.client_id = Some(client_id);
        query.include_subscriptions = true;
        query.include_auth = true;
        query
    }

    pub fn set_state(&mut self, state: ClientConnectionState) {
        if self.query.state != state {
            self.query.state = state;
            if !self.query.sort.is_allowed_for_state(state) {
                self.query.sort = ClientStatusSort::Cid;
            }
            self.query.offset = 0;
            self.clear_result_for_query_change();
        }
    }

    pub fn set_sort(&mut self, sort: ClientStatusSort) {
        let sort = if sort.is_allowed_for_state(self.query.state) {
            sort
        } else {
            ClientStatusSort::Cid
        };
        if self.query.sort != sort {
            self.query.sort = sort;
            self.clear_result_for_query_change();
        }
    }

    pub fn set_page_size(&mut self, page_size: usize) {
        let page_size = page_size.clamp(1, ClientStatusQuery::MAX_PAGE_SIZE);
        if self.query.page_size != page_size {
            self.query.page_size = page_size;
            self.query.offset = 0;
            self.clear_result_for_query_change();
        }
    }

    pub fn next_page(&mut self) {
        self.query.offset = self.query.offset.saturating_add(self.query.page_size);
        self.clear_result_for_query_change();
    }

    pub fn previous_page(&mut self) {
        self.query.offset = self.query.offset.saturating_sub(self.query.page_size);
        self.clear_result_for_query_change();
    }

    pub fn begin_page_refresh(&mut self) {
        self.loading = true;
        self.error = None;
        self.auto_refresh.mark_refreshed();
    }

    pub fn begin_detail_refresh(&mut self, client_id: u64) {
        self.select_client(client_id);
        self.detail_loading = true;
        self.error = None;
    }

    pub fn apply_page(&mut self, page: ClientStatusPage) {
        if !self.endpoint_matches(&page.endpoint) || page.query != self.query {
            return;
        }

        self.loading = false;
        self.error = None;
        if let Some(selected_id) = self.selected_client_id {
            let still_visible = page
                .clients
                .iter()
                .any(|client| client.client_id == selected_id);
            self.selected_detail_stale = !still_visible && self.detail.is_some();
        }
        self.page = Some(Box::new(page));
    }

    pub fn apply_detail(&mut self, detail: ClientStatusDetail) {
        let client_id = detail.query.client_id.unwrap_or(detail.client.client_id);
        if !self.endpoint_matches(&detail.endpoint) || self.selected_client_id != Some(client_id) {
            return;
        }

        self.detail_loading = false;
        self.selected_detail_stale = false;
        self.error = None;
        self.detail = Some(Box::new(detail));
    }

    pub fn apply_error(&mut self, error: ClientStatusRequestError) {
        if let Some(client_id) = error.query.client_id {
            if !self.endpoint_matches(&error.endpoint) || self.selected_client_id != Some(client_id)
            {
                return;
            }
            self.detail_loading = false;
            if self.detail.is_some() {
                self.selected_detail_stale = true;
            }
        } else {
            if !self.endpoint_matches(&error.endpoint) || error.query != self.query {
                return;
            }
            self.loading = false;
        }
        self.error = Some(Box::new(error));
    }

    pub fn select_client(&mut self, client_id: u64) {
        if self.selected_client_id == Some(client_id) {
            return;
        }
        self.selected_client_id = Some(client_id);
        self.detail = None;
        self.detail_loading = false;
        self.selected_detail_stale = false;
    }

    pub fn clear_selected_client(&mut self) {
        self.selected_client_id = None;
        self.detail = None;
        self.detail_loading = false;
        self.selected_detail_stale = false;
    }

    pub fn page(&self) -> Option<&ClientStatusPage> {
        self.page.as_deref()
    }

    pub fn detail(&self) -> Option<&ClientStatusDetail> {
        self.detail.as_deref()
    }

    pub fn error(&self) -> Option<&ClientStatusRequestError> {
        self.error.as_deref()
    }

    pub fn selected_client_id(&self) -> Option<u64> {
        self.selected_client_id
    }

    pub fn selected_detail_stale(&self) -> bool {
        self.selected_detail_stale
    }

    pub fn is_stale(&self) -> bool {
        self.error.is_some() && self.page.is_some()
    }

    pub fn should_refresh(&self) -> bool {
        self.endpoint_configured() && !self.loading && self.auto_refresh.should_refresh()
    }

    fn clear_status(&mut self) {
        self.query = ClientStatusQuery::default();
        self.page = None;
        self.detail = None;
        self.error = None;
        self.selected_client_id = None;
        self.selected_detail_stale = false;
        self.loading = false;
        self.detail_loading = false;
        self.auto_refresh.last_refresh = Instant::now();
    }

    fn clear_result_for_query_change(&mut self) {
        self.page = None;
        self.detail = None;
        self.error = None;
        self.selected_client_id = None;
        self.selected_detail_stale = false;
        self.loading = false;
        self.detail_loading = false;
    }

    fn endpoint_matches(&self, endpoint: &str) -> bool {
        normalize_monitoring_endpoint(endpoint) == self.endpoint
    }
}

fn normalize_monitoring_endpoint(endpoint: impl AsRef<str>) -> String {
    let endpoint = endpoint.as_ref();
    endpoint.trim().trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use nats_backend::{
        ClientConnectionState, ClientStatusDetail, ClientStatusPage, ClientStatusQuery,
        ClientStatusRequestError, ClientStatusRow, ClientStatusSort,
    };

    use super::ClientStatusState;

    fn sample_client_row(client_id: u64) -> ClientStatusRow {
        ClientStatusRow {
            client_id,
            state: ClientConnectionState::Open,
            name: Some(format!("client-{client_id}")),
            account: Some("APP".to_string()),
            user: Some("alice".to_string()),
            ip: Some("127.0.0.1".to_string()),
            port: Some(4200),
            uptime: Some("1m".to_string()),
            idle: Some("1s".to_string()),
            last_activity: Some("2026-05-07T10:00:00Z".to_string()),
            rtt: Some("1ms".to_string()),
            subscriptions: Some(3),
            pending_bytes: Some(0),
            in_msgs: Some(10),
            out_msgs: Some(11),
            in_bytes: Some(100),
            out_bytes: Some(110),
            language: Some("rust".to_string()),
            version: Some("1.0.0".to_string()),
            closed_at: None,
            closed_reason: None,
            subscription_details: Vec::new(),
        }
    }

    fn sample_client_page(endpoint: &str, clients: Vec<ClientStatusRow>) -> ClientStatusPage {
        ClientStatusPage {
            endpoint: endpoint.to_string(),
            collected_at: SystemTime::UNIX_EPOCH,
            query: ClientStatusQuery::default(),
            total: clients.len() as u64,
            offset: 0,
            limit: 100,
            clients,
        }
    }

    fn sample_client_page_with_query(
        endpoint: &str,
        query: ClientStatusQuery,
        clients: Vec<ClientStatusRow>,
    ) -> ClientStatusPage {
        ClientStatusPage {
            endpoint: endpoint.to_string(),
            collected_at: SystemTime::UNIX_EPOCH,
            query,
            total: clients.len() as u64,
            offset: 0,
            limit: 100,
            clients,
        }
    }

    fn sample_client_detail(endpoint: &str, client_id: u64) -> ClientStatusDetail {
        ClientStatusDetail {
            endpoint: endpoint.to_string(),
            collected_at: SystemTime::UNIX_EPOCH,
            query: ClientStatusQuery::detail(client_id),
            client: sample_client_row(client_id),
        }
    }

    #[test]
    fn client_status_state_clears_result_when_endpoint_changes() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.apply_page(sample_client_page(
            "http://localhost:8222",
            vec![sample_client_row(1)],
        ));
        state.select_client(1);
        state.apply_detail(sample_client_detail("http://localhost:8222", 1));

        state.set_endpoint("http://localhost:9222".to_string());

        assert_eq!(state.endpoint(), "http://localhost:9222");
        assert!(state.page().is_none());
        assert!(state.detail().is_none());
        assert_eq!(state.selected_client_id(), None);
    }

    #[test]
    fn client_status_endpoint_is_normalized_for_page_responses() {
        let mut state = ClientStatusState::with_endpoint(" http://localhost:8222/ ".to_string());
        state.begin_page_refresh();

        state.apply_page(sample_client_page(
            "http://localhost:8222",
            vec![sample_client_row(1)],
        ));

        assert_eq!(state.endpoint(), "http://localhost:8222");
        assert_eq!(state.page().unwrap().clients.len(), 1);
        assert!(!state.loading);
    }

    #[test]
    fn stale_client_status_page_response_is_ignored() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.set_state(ClientConnectionState::Closed);
        state.begin_page_refresh();

        state.apply_page(sample_client_page(
            "http://localhost:8222",
            vec![sample_client_row(1)],
        ));

        assert_eq!(state.query().state, ClientConnectionState::Closed);
        assert!(state.page().is_none());
        assert!(state.loading);
    }

    #[test]
    fn stale_client_status_page_response_from_old_endpoint_is_ignored() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.set_endpoint("http://localhost:9222".to_string());
        state.begin_page_refresh();

        state.apply_page(sample_client_page(
            "http://localhost:8222",
            vec![sample_client_row(1)],
        ));

        assert_eq!(state.endpoint(), "http://localhost:9222");
        assert!(state.page().is_none());
        assert!(state.loading);
    }

    #[test]
    fn matching_client_status_page_response_is_applied() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.set_state(ClientConnectionState::Closed);
        state.begin_page_refresh();

        state.apply_page(sample_client_page_with_query(
            "http://localhost:8222",
            state.query().clone(),
            vec![sample_client_row(1)],
        ));

        assert_eq!(state.query().state, ClientConnectionState::Closed);
        assert_eq!(state.page().unwrap().clients.len(), 1);
        assert!(!state.loading);
    }

    #[test]
    fn selected_client_detail_becomes_stale_when_refreshed_page_omits_client() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.apply_page(sample_client_page(
            "http://localhost:8222",
            vec![sample_client_row(1)],
        ));
        state.select_client(1);
        state.apply_detail(sample_client_detail("http://localhost:8222", 1));

        state.apply_page(sample_client_page(
            "http://localhost:8222",
            vec![sample_client_row(2)],
        ));

        assert_eq!(state.selected_client_id(), Some(1));
        assert!(state.selected_detail_stale());
        assert_eq!(state.detail().unwrap().client.client_id, 1);
    }

    #[test]
    fn stale_client_detail_response_is_ignored_after_clear() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.begin_detail_refresh(1);
        state.clear_selected_client();

        state.apply_detail(sample_client_detail("http://localhost:8222", 1));

        assert_eq!(state.selected_client_id(), None);
        assert!(state.detail().is_none());
        assert!(!state.detail_loading);
    }

    #[test]
    fn stale_client_detail_response_does_not_overwrite_new_selection() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.begin_detail_refresh(1);
        state.begin_detail_refresh(2);

        state.apply_detail(sample_client_detail("http://localhost:8222", 1));

        assert_eq!(state.selected_client_id(), Some(2));
        assert!(state.detail().is_none());
        assert!(state.detail_loading);
    }

    #[test]
    fn stale_client_detail_response_from_old_endpoint_is_ignored() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.set_endpoint("http://localhost:9222".to_string());
        state.begin_detail_refresh(1);

        state.apply_detail(sample_client_detail("http://localhost:8222", 1));

        assert_eq!(state.endpoint(), "http://localhost:9222");
        assert_eq!(state.selected_client_id(), Some(1));
        assert!(state.detail().is_none());
        assert!(state.detail_loading);
    }

    #[test]
    fn client_status_error_does_not_clear_last_page() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.apply_page(sample_client_page(
            "http://localhost:8222",
            vec![sample_client_row(1)],
        ));

        state.apply_error(ClientStatusRequestError {
            endpoint: "http://localhost:8222".to_string(),
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
            query: ClientStatusQuery::default(),
            message: "timeout".to_string(),
        });

        assert_eq!(state.page().unwrap().clients.len(), 1);
        assert_eq!(state.error().unwrap().message, "timeout");
        assert!(state.is_stale());
    }

    #[test]
    fn stale_client_status_page_error_is_ignored() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.set_state(ClientConnectionState::Closed);
        state.begin_page_refresh();

        state.apply_error(ClientStatusRequestError {
            endpoint: "http://localhost:8222".to_string(),
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
            query: ClientStatusQuery::default(),
            message: "timeout".to_string(),
        });

        assert!(state.error().is_none());
        assert!(state.loading);
    }

    #[test]
    fn stale_client_status_page_error_from_old_endpoint_is_ignored() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.set_endpoint("http://localhost:9222".to_string());
        state.begin_page_refresh();

        state.apply_error(ClientStatusRequestError {
            endpoint: "http://localhost:8222".to_string(),
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
            query: ClientStatusQuery::default(),
            message: "timeout".to_string(),
        });

        assert!(state.error().is_none());
        assert!(state.loading);
    }

    #[test]
    fn client_status_detail_error_clears_detail_loading() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.begin_detail_refresh(1);

        state.apply_error(ClientStatusRequestError {
            endpoint: "http://localhost:8222".to_string(),
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
            query: ClientStatusQuery::detail(1),
            message: "missing".to_string(),
        });

        assert!(!state.detail_loading);
        assert_eq!(state.selected_client_id(), Some(1));
        assert_eq!(state.error().unwrap().message, "missing");
    }

    #[test]
    fn stale_client_status_detail_error_is_ignored() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.begin_detail_refresh(1);
        state.begin_detail_refresh(2);

        state.apply_error(ClientStatusRequestError {
            endpoint: "http://localhost:8222".to_string(),
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
            query: ClientStatusQuery::detail(1),
            message: "missing".to_string(),
        });

        assert!(state.error().is_none());
        assert_eq!(state.selected_client_id(), Some(2));
        assert!(state.detail_loading);
    }

    #[test]
    fn stale_client_status_detail_error_from_old_endpoint_is_ignored() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.set_endpoint("http://localhost:9222".to_string());
        state.begin_detail_refresh(1);

        state.apply_error(ClientStatusRequestError {
            endpoint: "http://localhost:8222".to_string(),
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
            query: ClientStatusQuery::detail(1),
            message: "missing".to_string(),
        });

        assert!(state.error().is_none());
        assert_eq!(state.selected_client_id(), Some(1));
        assert!(state.detail_loading);
    }

    #[test]
    fn client_status_query_change_resets_page_and_selection() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.apply_page(sample_client_page(
            "http://localhost:8222",
            vec![sample_client_row(1)],
        ));
        state.select_client(1);

        state.set_state(ClientConnectionState::Closed);

        assert_eq!(state.query().state, ClientConnectionState::Closed);
        assert_eq!(state.query().offset, 0);
        assert!(state.page().is_none());
        assert_eq!(state.selected_client_id(), None);
    }

    #[test]
    fn client_status_detail_query_preserves_current_state() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.set_state(ClientConnectionState::Closed);

        let query = state.detail_query(7);

        assert_eq!(query.client_id, Some(7));
        assert_eq!(query.state, ClientConnectionState::Closed);
        assert!(query.include_subscriptions);
        assert!(query.include_auth);
    }

    #[test]
    fn client_status_rejects_closed_only_sort_outside_closed_state() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());

        state.set_sort(ClientStatusSort::Stop);

        assert_eq!(state.query().sort, ClientStatusSort::Cid);
    }

    #[test]
    fn client_status_resets_closed_only_sort_when_leaving_closed_state() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.set_state(ClientConnectionState::Closed);
        state.set_sort(ClientStatusSort::Reason);

        state.set_state(ClientConnectionState::Any);

        assert_eq!(state.query().state, ClientConnectionState::Any);
        assert_eq!(state.query().sort, ClientStatusSort::Cid);
    }

    #[test]
    fn client_status_auto_refresh_uses_own_timer() {
        let mut state = ClientStatusState::with_endpoint("http://localhost:8222".to_string());
        state.auto_refresh.enabled = true;
        state.auto_refresh.interval_secs = 0;

        assert!(state.should_refresh());

        state.loading = true;

        assert!(!state.should_refresh());
    }
}
