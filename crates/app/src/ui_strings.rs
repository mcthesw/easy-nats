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

// ─── Consumer ───
pub const CONSUMER_HEADING: &str = "Consumers";
pub const CONSUMER_REFRESH: &str = "Refresh";
pub const CONSUMER_CREATE: &str = "Create Consumer";
pub const CONSUMER_LOADING: &str = "Loading consumers...";
pub const CONSUMER_NO_CONSUMERS: &str = "No consumers. Click Create to add one.";
pub const CONSUMER_STREAM: &str = "Stream:";
pub const CONSUMER_NAME: &str = "Name:";
pub const CONSUMER_TYPE: &str = "Type:";
pub const CONSUMER_TYPE_PULL: &str = "Pull";
pub const CONSUMER_TYPE_PUSH: &str = "Push";
pub const CONSUMER_DURABLE_MODE: &str = "Durability:";
pub const CONSUMER_DURABLE_CHECKBOX: &str = "Durable consumer";
pub const CONSUMER_PENDING: &str = "Pending:";
pub const CONSUMER_ACK_PENDING: &str = "Ack pending:";
pub const CONSUMER_WAITING: &str = "Waiting:";
pub const CONSUMER_REDELIVERED: &str = "Redelivered:";
pub const CONSUMER_DELETE: &str = "Delete";
pub const CONSUMER_DURABLE: &str = "Durable name:";
pub const CONSUMER_FILTER_SUBJECT: &str = "Filter subject:";
pub const CONSUMER_ACK_POLICY: &str = "Ack policy:";
pub const CONSUMER_DELIVER_POLICY: &str = "Deliver policy:";
pub const CONSUMER_MAX_DELIVER: &str = "Max deliver:";
pub const CONSUMER_MAX_ACK_PENDING: &str = "Max ack pending:";
pub const CONSUMER_DESCRIPTION: &str = "Description:";
pub const CONSUMER_POLICY_ALL: &str = "All";
pub const CONSUMER_POLICY_LAST: &str = "Last";
pub const CONSUMER_POLICY_NEW: &str = "New";
pub const CONSUMER_ACK_EXPLICIT: &str = "Explicit";
pub const CONSUMER_ACK_ALL: &str = "All";
pub const CONSUMER_ACK_NONE: &str = "None";

// ─── KV Store ───
pub const KV_BUCKET_INFO: &str = "Bucket Info";
pub const KV_CREATE_BUCKET: &str = "Create KV Bucket";
pub const KV_DELETE_BUCKET: &str = "Delete Bucket";
pub const KV_DELETE_BUCKET_CONFIRM_TITLE: &str = "Confirm KV Bucket Delete";
pub const KV_DELETE_BUCKET_CONFIRM_PROMPT: &str = "Delete KV bucket";
pub const KV_BUCKET: &str = "Bucket:";
pub const KV_VALUES: &str = "Values:";
pub const KV_HISTORY_DEPTH: &str = "History depth:";
pub const KV_MAX_AGE: &str = "Max age (sec):";
pub const KV_MAX_VALUE_SIZE: &str = "Max value size:";
pub const KV_MAX_BYTES: &str = "Max bytes:";
pub const KV_STORAGE: &str = "Storage:";
pub const KV_REPLICAS: &str = "Replicas:";
pub const KV_DESCRIPTION: &str = "Description:";
pub const KV_BYTES: &str = "Size:";
pub const KV_KEY_FILTER: &str = "Key filter:";
pub const KV_REFRESH: &str = "Refresh";
pub const KV_NEW_ENTRY: &str = "New Entry";
pub const KV_LOADING_KEYS: &str = "Loading keys...";
pub const KV_KEYS: &str = "Keys";
pub const KV_NO_KEYS: &str = "No keys found in this bucket.";
pub const KV_KEY: &str = "Key:";
pub const KV_REVISION: &str = "Revision:";
pub const KV_OPERATION: &str = "Operation:";
pub const KV_CREATED: &str = "Created:";
pub const KV_VALUE_EDITOR: &str = "Value editor";
pub const KV_VALUE_PREVIEW: &str = "Value preview";
pub const KV_DELETE_ENTRY: &str = "Delete Entry";
pub const KV_PURGE_ENTRY: &str = "Purge Entry";
pub const KV_HISTORY: &str = "History";
pub const KV_LOADING_HISTORY: &str = "Loading history...";

// ─── Object Store ───
pub const OBJECT_STORE_WIP: &str = "🚧 Object Store is under construction.";

// ─── Storage / Retention Labels ───
pub const STORAGE_FILE: &str = "File";
pub const STORAGE_MEMORY: &str = "Memory";
pub const RETENTION_LIMITS: &str = "Limits";
pub const RETENTION_INTEREST: &str = "Interest";
pub const RETENTION_WORK_QUEUE: &str = "WorkQueue";

// ─── Toast / Result Messages ───
pub const TOAST_STREAM_DELETED: &str = "Stream deleted";
pub const TOAST_MESSAGE_DELETED: &str = "Message deleted";

// ─── Misc ───
pub const STREAM_REFRESH: &str = "Refresh";
pub const STREAM_INVALID_BASE64: &str = "Invalid base64 payload";
pub const STREAM_NO_PAYLOAD: &str = "No payload";
pub const CONSUMER_UNNAMED: &str = "(unnamed)";
pub const KV_NO_HISTORY: &str = "No history loaded yet.";
pub const KV_NONE: &str = "—";
pub const KV_EMPTY_VALUE: &str = "<empty>";
