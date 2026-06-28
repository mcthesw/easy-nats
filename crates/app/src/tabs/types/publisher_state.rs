use nats_backend::RequestFailureKind;

use crate::format::PayloadFormat;
use crate::proto::ProtoViewState;
use crate::schema::PayloadInputFormat;

#[derive(Debug)]
pub struct PublisherState {
    pub subject: String,
    pub subject_suggestion_idx: Option<usize>,
    pub payload: String,
    pub headers: Vec<(String, String)>,
    pub timeout_ms: String,
    pub current_request: Option<CurrentRequest>,
    pub next_request_id: u64,
    pub payload_input_format: PayloadInputFormat,
    pub response_format: PayloadFormat,
    pub proto_view: ProtoViewState,
}

impl Default for PublisherState {
    fn default() -> Self {
        Self {
            subject: String::new(),
            subject_suggestion_idx: None,
            payload: String::new(),
            headers: Vec::new(),
            timeout_ms: "5000".to_string(),
            current_request: None,
            next_request_id: 1,
            payload_input_format: PayloadInputFormat::Text,
            response_format: PayloadFormat::Auto,
            proto_view: ProtoViewState::default(),
        }
    }
}

impl PublisherState {
    pub fn start_request(&mut self, subject: String) -> u64 {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        self.current_request = Some(CurrentRequest {
            request_id,
            subject,
            status: RequestStatus::Waiting,
            response: None,
            error_message: None,
        });
        request_id
    }

    pub fn apply_request_response(&mut self, request_id: u64, response: ResponseData) {
        let Some(current) = self.current_request.as_mut() else {
            return;
        };
        if current.request_id != request_id {
            return;
        }
        current.status = RequestStatus::Responded;
        current.response = Some(response);
        current.error_message = None;
    }

    pub fn apply_request_failure(
        &mut self,
        request_id: u64,
        kind: RequestFailureKind,
        message: String,
    ) {
        let Some(current) = self.current_request.as_mut() else {
            return;
        };
        if current.request_id != request_id {
            return;
        }
        current.status = RequestStatus::from_failure_kind(kind);
        current.response = None;
        current.error_message = Some(message);
    }

    pub fn is_request_waiting(&self) -> bool {
        self.current_request
            .as_ref()
            .is_some_and(|request| request.status == RequestStatus::Waiting)
    }
}

#[derive(Debug, Clone)]
pub struct CurrentRequest {
    pub request_id: u64,
    pub subject: String,
    pub status: RequestStatus,
    pub response: Option<ResponseData>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResponseData {
    pub subject: Option<String>,
    pub payload: Vec<u8>,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStatus {
    Waiting,
    Responded,
    TimedOut,
    NoResponders,
    Failed,
}

impl RequestStatus {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Waiting => "publisher.request_status_waiting",
            Self::Responded => "publisher.request_status_responded",
            Self::TimedOut => "publisher.request_status_timed_out",
            Self::NoResponders => "publisher.request_status_no_responders",
            Self::Failed => "publisher.request_status_failed",
        }
    }

    fn from_failure_kind(kind: RequestFailureKind) -> Self {
        match kind {
            RequestFailureKind::TimedOut => Self::TimedOut,
            RequestFailureKind::NoResponders => Self::NoResponders,
            RequestFailureKind::Other => Self::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_slot_ignores_stale_results() {
        let mut state = PublisherState::default();
        let first = state.start_request("orders.created".to_string());
        let second = state.start_request("orders.updated".to_string());

        state.apply_request_response(
            first,
            ResponseData {
                subject: Some("orders.created".to_string()),
                payload: b"stale".to_vec(),
                headers: Vec::new(),
            },
        );

        let current = state.current_request.as_ref().expect("current request");
        assert_eq!(current.request_id, second);
        assert_eq!(current.status, RequestStatus::Waiting);
        assert!(current.response.is_none());
    }

    #[test]
    fn request_failure_maps_kind_for_current_request() {
        let mut state = PublisherState::default();
        let request_id = state.start_request("orders.created".to_string());

        state.apply_request_failure(
            request_id,
            RequestFailureKind::NoResponders,
            "no responders".to_string(),
        );

        let current = state.current_request.as_ref().expect("current request");
        assert_eq!(current.status, RequestStatus::NoResponders);
        assert_eq!(current.error_message.as_deref(), Some("no responders"));
    }
}
