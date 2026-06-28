use std::collections::{HashMap, VecDeque};
use std::time::SystemTime;

use crate::format::PayloadFormat;
use crate::proto::ProtoViewState;
use crate::schema::PayloadInputFormat;

use super::{ScopedSearchState, SearchCacheKey};

#[derive(Debug, Clone)]
pub struct ReceivedMessage {
    pub id: u64,
    pub subject: String,
    pub reply: Option<String>,
    pub headers: Vec<(String, String)>,
    pub payload: Vec<u8>,
    pub timestamp: SystemTime,
    pub reply_state: Option<ReplyState>,
    pub reply_draft: Option<ReplyDraft>,
}

impl ReceivedMessage {
    pub fn new(
        subject: String,
        reply: Option<String>,
        headers: Vec<(String, String)>,
        payload: Vec<u8>,
        timestamp: SystemTime,
    ) -> Self {
        let is_replyable = reply.is_some();
        Self {
            id: 0,
            subject,
            reply,
            headers,
            payload,
            timestamp,
            reply_state: is_replyable.then_some(ReplyState::Replyable),
            reply_draft: is_replyable.then(ReplyDraft::default),
        }
    }

    pub fn reply_list_status(&self) -> Option<ReplyListStatus> {
        match self.reply_state.as_ref()? {
            ReplyState::Replyable => Some(ReplyListStatus::Replyable),
            ReplyState::Sending { .. } => Some(ReplyListStatus::Sending),
            ReplyState::Replied => Some(ReplyListStatus::Replied),
            ReplyState::Failed(_) => Some(ReplyListStatus::Failed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplyDraft {
    pub payload: String,
    pub headers: Vec<(String, String)>,
    pub payload_input_format: PayloadInputFormat,
}

impl Default for ReplyDraft {
    fn default() -> Self {
        Self {
            payload: String::new(),
            headers: Vec::new(),
            payload_input_format: PayloadInputFormat::Text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplyState {
    Replyable,
    Sending { reply_id: u64 },
    Replied,
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyListStatus {
    Replyable,
    Sending,
    Replied,
    Failed,
}

impl ReplyListStatus {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Replyable => "subscriber.reply_status_replyable",
            Self::Sending => "subscriber.reply_status_sending",
            Self::Replied => "subscriber.reply_status_replied",
            Self::Failed => "subscriber.reply_status_failed",
        }
    }
}

pub type SubscriberListRow = (usize, String, String, Option<ReplyListStatus>);
pub type CachedSubscriberRows = (u64, Option<String>, SearchCacheKey, Vec<SubscriberListRow>);

#[derive(Debug)]
pub struct SubjectSubscription {
    pub subject: String,
    pub active: bool,
}

#[derive(Debug)]
pub struct SubscriberState {
    pub subject_input: String,
    pub subject_suggestion_idx: Option<usize>,
    pub subscriptions: Vec<SubjectSubscription>,
    pub messages: VecDeque<ReceivedMessage>,
    pub next_message_id: u64,
    pub next_reply_id: u64,
    pub in_flight_replies: HashMap<u64, u64>,
    pub max_messages: usize,
    pub selected_idx: Option<usize>,
    pub payload_format: PayloadFormat,
    /// When set, only display messages matching this subject.
    pub subject_filter: Option<String>,
    pub cache_generation: u64,
    pub cached_filtered: Option<CachedSubscriberRows>,
    pub search: ScopedSearchState,
    pub proto_view: ProtoViewState,
}

impl Default for SubscriberState {
    fn default() -> Self {
        Self {
            subject_input: String::new(),
            subject_suggestion_idx: None,
            subscriptions: Vec::new(),
            messages: VecDeque::new(),
            next_message_id: 1,
            next_reply_id: 1,
            in_flight_replies: HashMap::new(),
            max_messages: 1000,
            selected_idx: None,
            payload_format: PayloadFormat::Auto,
            subject_filter: None,
            cache_generation: 0,
            cached_filtered: None,
            search: ScopedSearchState::default(),
            proto_view: ProtoViewState::default(),
        }
    }
}

impl SubscriberState {
    pub fn push_messages<I>(&mut self, messages: I)
    where
        I: IntoIterator<Item = ReceivedMessage>,
    {
        let mut pushed = false;
        for msg in messages {
            self.push_message_without_invalidation(msg);
            pushed = true;
        }
        if pushed {
            self.invalidate_filtered_cache();
        }
    }

    fn push_message_without_invalidation(&mut self, mut msg: ReceivedMessage) {
        if self.messages.len() >= self.max_messages {
            if let Some(removed) = self.messages.pop_front() {
                self.forget_in_flight_reply_for_message(&removed);
            }
            if let Some(idx) = self.selected_idx {
                self.selected_idx = idx.checked_sub(1);
            }
        }
        msg.id = self.next_message_id;
        self.next_message_id = self.next_message_id.wrapping_add(1).max(1);
        self.messages.push_back(msg);
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.in_flight_replies.clear();
        self.selected_idx = None;
        self.invalidate_filtered_cache();
    }

    pub fn begin_reply(&mut self, message_id: u64) -> Option<u64> {
        let message = self
            .messages
            .iter_mut()
            .find(|message| message.id == message_id)?;
        message.reply.as_ref()?;
        if matches!(
            message.reply_state,
            Some(ReplyState::Sending { .. }) | Some(ReplyState::Replied)
        ) {
            return None;
        }
        let reply_id = self.next_reply_id;
        self.next_reply_id = self.next_reply_id.wrapping_add(1).max(1);
        message.reply_state = Some(ReplyState::Sending { reply_id });
        self.in_flight_replies.insert(reply_id, message_id);
        self.invalidate_filtered_cache();
        Some(reply_id)
    }

    pub fn apply_reply_success(&mut self, reply_id: u64) {
        let Some(message_id) = self.in_flight_replies.remove(&reply_id) else {
            return;
        };
        if let Some(message) = self.message_mut(message_id) {
            message.reply_state = Some(ReplyState::Replied);
        }
        self.invalidate_filtered_cache();
    }

    pub fn apply_reply_failure(&mut self, reply_id: u64, message: String) {
        let Some(message_id) = self.in_flight_replies.remove(&reply_id) else {
            return;
        };
        if let Some(message_ref) = self.message_mut(message_id) {
            message_ref.reply_state = Some(ReplyState::Failed(message));
        }
        self.invalidate_filtered_cache();
    }

    pub fn message_mut(&mut self, message_id: u64) -> Option<&mut ReceivedMessage> {
        self.messages
            .iter_mut()
            .find(|message| message.id == message_id)
    }

    pub fn invalidate_filtered_cache(&mut self) {
        self.cache_generation = self.cache_generation.wrapping_add(1);
        self.cached_filtered = None;
    }

    fn forget_in_flight_reply_for_message(&mut self, message: &ReceivedMessage) {
        if let Some(ReplyState::Sending { reply_id }) = &message.reply_state {
            self.in_flight_replies.remove(reply_id);
        }
    }
}
