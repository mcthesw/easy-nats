// Centralized UI strings for all user-facing text.
// Organized by feature area for future i18n readiness.

// ─── Connections ───
pub const CONNECTIONS_HEADING: &str = "Connections";
pub const CONNECTION_NEW: &str = "New Connection";
pub const CONNECTION_EDIT: &str = "Edit Connection";
pub const CONNECTION_DELETE_CONFIRM_TITLE: &str = "Confirm Delete";
pub const CONNECTION_DELETE_PROMPT: &str = "Delete connection";
pub const CONNECT: &str = "Connect";
pub const DISCONNECT: &str = "Disconnect";

// ─── Editor Fields ───
pub const FIELD_NAME: &str = "Name:";
pub const FIELD_URL: &str = "URL:";
pub const FIELD_AUTH: &str = "Auth:";
pub const FIELD_TOKEN: &str = "Token:";
pub const FIELD_USERNAME: &str = "Username:";
pub const FIELD_PASSWORD: &str = "Password:";
pub const FIELD_NKEY_SEED: &str = "NKey Seed:";
pub const FIELD_CREDS_FILE: &str = "Creds File:";
pub const FIELD_CERT_PATH: &str = "Cert Path:";
pub const FIELD_KEY_PATH: &str = "Key Path:";
pub const FIELD_TLS: &str = "TLS:";
pub const REQUIRE_TLS: &str = "Require TLS";

// ─── Auth Methods ───
pub const AUTH_NONE: &str = "No Auth";
pub const AUTH_TOKEN: &str = "Token";
pub const AUTH_USER_PASSWORD: &str = "User / Password";
pub const AUTH_NKEY: &str = "NKey";
pub const AUTH_CREDENTIALS_FILE: &str = "Credentials File";
pub const AUTH_TLS_CLIENT_CERT: &str = "TLS Client Certificate";

// ─── Actions ───
pub const SAVE: &str = "Save";
pub const CANCEL: &str = "Cancel";
pub const DELETE: &str = "Delete";
pub const EDIT: &str = "Edit";

// ─── Sidebar Resource Tree ───
pub const SECTION_PUBSUB: &str = "Pub/Sub";
pub const SECTION_STREAMS: &str = "Streams";
pub const SECTION_KV: &str = "Key-Value";
pub const SECTION_OBJECT_STORE: &str = "Object Store";
pub const OPEN_PUBLISHER: &str = "Publisher";
pub const OPEN_SUBSCRIBER: &str = "Subscriber";

// ─── Tabs ───
pub const TAB_WELCOME: &str = "Welcome";
pub const TAB_PUBLISHER: &str = "Publisher";
pub const TAB_SUBSCRIBER: &str = "Subscriber";
#[allow(dead_code)]
pub const TAB_STREAM: &str = "Stream";
#[allow(dead_code)]
pub const TAB_KV_BUCKET: &str = "KV Bucket";
#[allow(dead_code)]
pub const TAB_OBJECT_STORE_BUCKET: &str = "Object Store";

// ─── Theme ───
pub const THEME_DARK: &str = "🌙";
pub const THEME_LIGHT: &str = "☀";

// ─── Toast ───
pub const TOAST_DISMISS: &str = "✕";

// ─── Welcome ───
pub const WELCOME_HEADING: &str = "Easy NATS";
pub const WELCOME_BODY: &str = "Create or select a connection from the sidebar to get started.\nUse the resource tree to open publisher, subscriber, stream, KV, or object store tabs.";

// ─── Publisher ───
pub const PUBLISHER_SUBJECT: &str = "Subject:";
pub const PUBLISHER_PAYLOAD: &str = "Payload";
pub const PUBLISHER_HEADERS: &str = "Headers";
pub const PUBLISHER_HEADER_KEY: &str = "Key";
pub const PUBLISHER_HEADER_VALUE: &str = "Value";
pub const PUBLISHER_ADD_HEADER: &str = "+ Header";
pub const PUBLISHER_PUBLISH: &str = "Publish";
pub const PUBLISHER_REQUEST: &str = "Request";
pub const PUBLISHER_TIMEOUT: &str = "Timeout (ms):";
pub const PUBLISHER_RESPONSE: &str = "Response";
pub const PUBLISHER_NO_RESPONSE: &str = "No response yet. Use \"Request\" to send a request-reply.";
pub const PUBLISHER_RESPONSE_PAYLOAD: &str = "Payload:";
pub const PUBLISHER_RESPONSE_HEADERS: &str = "Headers:";
pub const PUBLISHER_WAITING: &str = "Waiting for response...";

// ─── Subscriber ───
pub const SUBSCRIBER_SUBJECT: &str = "Subject:";
pub const SUBSCRIBER_SUBSCRIBE: &str = "Subscribe";
pub const SUBSCRIBER_UNSUBSCRIBE: &str = "Unsubscribe";
pub const SUBSCRIBER_MSG_COUNT: &str = "Messages:";
pub const SUBSCRIBER_CLEAR: &str = "Clear";
pub const SUBSCRIBER_MESSAGES: &str = "Messages";
pub const SUBSCRIBER_NO_MESSAGES: &str = "No messages received yet.";
pub const SUBSCRIBER_SELECT_MSG: &str = "Select a message to view details.";
pub const SUBSCRIBER_DETAIL: &str = "Message Detail";
pub const SUBSCRIBER_DETAIL_SUBJECT: &str = "Subject:";
pub const SUBSCRIBER_DETAIL_REPLY: &str = "Reply-To:";
pub const SUBSCRIBER_DETAIL_TIMESTAMP: &str = "Timestamp:";
pub const SUBSCRIBER_DETAIL_SIZE: &str = "Size:";
pub const SUBSCRIBER_DETAIL_HEADERS: &str = "Headers:";
pub const SUBSCRIBER_DETAIL_PAYLOAD: &str = "Payload:";

// ─── Stream ───
pub const STREAM_INFO: &str = "Stream Info";
pub const STREAM_NAME: &str = "Name:";
pub const STREAM_SUBJECTS: &str = "Subjects:";
pub const STREAM_STORAGE: &str = "Storage:";
pub const STREAM_RETENTION: &str = "Retention:";
pub const STREAM_MSG_COUNT: &str = "Messages:";
pub const STREAM_BYTES: &str = "Size:";
pub const STREAM_CONSUMERS: &str = "Consumers:";
pub const STREAM_MESSAGES: &str = "Messages";
pub const STREAM_NO_MESSAGES: &str = "No messages fetched. Click Fetch to load messages.";
pub const STREAM_SELECT_MSG: &str = "Select a message to view details.";
pub const STREAM_START_SEQ: &str = "Start seq:";
pub const STREAM_SUBJECT_FILTER: &str = "Subject:";
pub const STREAM_BATCH_SIZE: &str = "Batch:";
pub const STREAM_FETCH: &str = "Fetch";
pub const STREAM_MSG_DETAIL: &str = "Message Detail";
pub const STREAM_MSG_SEQUENCE: &str = "Sequence:";
pub const STREAM_MSG_SUBJECT: &str = "Subject:";
pub const STREAM_MSG_HEADERS: &str = "Headers:";
pub const STREAM_MSG_PAYLOAD: &str = "Payload:";
pub const STREAM_PURGE: &str = "Purge";
pub const STREAM_PURGE_SUBJECT: &str = "Subject filter:";
pub const STREAM_PURGE_FILTERED: &str = "Purge Filtered";
pub const STREAM_PURGE_ALL: &str = "Purge All";
pub const STREAM_DELETE_MSG: &str = "Delete message";
pub const STREAM_CREATE_TITLE: &str = "Create Stream";
pub const STREAM_MAX_MSGS: &str = "Max messages:";
pub const STREAM_MAX_BYTES: &str = "Max bytes:";
pub const STREAM_MAX_AGE: &str = "Max age (sec):";
pub const STREAM_REPLICAS: &str = "Replicas:";
pub const STREAM_DESCRIPTION: &str = "Description:";
