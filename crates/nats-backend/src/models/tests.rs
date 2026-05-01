use super::*;

#[test]
fn stream_config_input_maps_to_async_nats_config() {
    let config = StreamConfigInput {
        name: "orders".to_string(),
        subjects: vec!["orders.*".to_string()],
        storage: StorageKind::Memory,
        retention: StreamRetentionKind::WorkQueue,
        max_messages: Some(42),
        max_bytes: Some(1024),
        max_age: Some(Duration::from_secs(5)),
        num_replicas: Some(3),
        description: Some("order events".to_string()),
    }
    .into_async_nats();

    assert_eq!(config.name, "orders");
    assert_eq!(config.subjects, vec!["orders.*"]);
    assert_eq!(
        config.storage,
        async_nats::jetstream::stream::StorageType::Memory
    );
    assert_eq!(
        config.retention,
        async_nats::jetstream::stream::RetentionPolicy::WorkQueue
    );
    assert_eq!(config.max_messages, 42);
    assert_eq!(config.max_bytes, 1024);
    assert_eq!(config.max_age, Duration::from_secs(5));
    assert_eq!(config.num_replicas, 3);
    assert_eq!(config.description.as_deref(), Some("order events"));
}

#[test]
fn consumer_config_input_maps_to_async_nats_pull_config() {
    let config = ConsumerConfigInput {
        name: "durable".to_string(),
        durable_name: Some("durable".to_string()),
        filter_subject: Some("orders.created".to_string()),
        deliver_policy: ConsumerDeliverPolicyKind::New,
        ack_policy: ConsumerAckPolicyKind::All,
        max_deliver: Some(5),
        max_ack_pending: Some(128),
        description: Some("worker".to_string()),
    }
    .into_async_nats_pull()
    .expect("valid consumer config");

    assert_eq!(config.name.as_deref(), Some("durable"));
    assert_eq!(config.durable_name.as_deref(), Some("durable"));
    assert_eq!(config.filter_subject, "orders.created");
    assert_eq!(
        config.deliver_policy,
        async_nats::jetstream::consumer::DeliverPolicy::New
    );
    assert_eq!(
        config.ack_policy,
        async_nats::jetstream::consumer::AckPolicy::All
    );
    assert_eq!(config.max_deliver, 5);
    assert_eq!(config.max_ack_pending, 128);
    assert_eq!(config.description.as_deref(), Some("worker"));
}

#[test]
fn consumer_deliver_policy_supports_all_pull_variants() {
    let by_sequence = ConsumerConfigInput {
        name: "sequence".to_string(),
        durable_name: Some("sequence".to_string()),
        filter_subject: None,
        deliver_policy: ConsumerDeliverPolicyKind::ByStartSequence { start_sequence: 42 },
        ack_policy: ConsumerAckPolicyKind::Explicit,
        max_deliver: None,
        max_ack_pending: None,
        description: None,
    }
    .into_async_nats_pull()
    .expect("valid sequence policy");
    assert_eq!(
        by_sequence.deliver_policy,
        async_nats::jetstream::consumer::DeliverPolicy::ByStartSequence { start_sequence: 42 }
    );

    let by_time = ConsumerConfigInput {
        name: "time".to_string(),
        durable_name: Some("time".to_string()),
        filter_subject: None,
        deliver_policy: ConsumerDeliverPolicyKind::ByStartTime {
            start_time: "1970-01-01T00:00:00Z".to_string(),
        },
        ack_policy: ConsumerAckPolicyKind::Explicit,
        max_deliver: None,
        max_ack_pending: None,
        description: None,
    }
    .into_async_nats_pull()
    .expect("valid start time policy");
    assert!(matches!(
        by_time.deliver_policy,
        async_nats::jetstream::consumer::DeliverPolicy::ByStartTime { .. }
    ));

    let last_per_subject = ConsumerConfigInput {
        name: "last-per-subject".to_string(),
        durable_name: Some("last-per-subject".to_string()),
        filter_subject: None,
        deliver_policy: ConsumerDeliverPolicyKind::LastPerSubject,
        ack_policy: ConsumerAckPolicyKind::Explicit,
        max_deliver: None,
        max_ack_pending: None,
        description: None,
    }
    .into_async_nats_pull()
    .expect("valid last-per-subject policy");
    assert_eq!(
        last_per_subject.deliver_policy,
        async_nats::jetstream::consumer::DeliverPolicy::LastPerSubject
    );
}

#[test]
fn consumer_deliver_policy_rejects_invalid_start_time() {
    let result = ConsumerConfigInput {
        name: "time".to_string(),
        durable_name: Some("time".to_string()),
        filter_subject: None,
        deliver_policy: ConsumerDeliverPolicyKind::ByStartTime {
            start_time: "not-a-time".to_string(),
        },
        ack_policy: ConsumerAckPolicyKind::Explicit,
        max_deliver: None,
        max_ack_pending: None,
        description: None,
    }
    .into_async_nats_pull();

    assert!(result.is_err());
}

#[test]
fn bucket_config_inputs_map_to_async_nats_configs() {
    let kv = KvBucketConfigInput {
        bucket: "settings".to_string(),
        history: 3,
        storage: StorageKind::File,
        max_value_size: Some(2048),
        max_bytes: Some(4096),
        max_age: Some(Duration::from_secs(60)),
        num_replicas: Some(2),
        description: Some("app settings".to_string()),
    }
    .into_async_nats();
    assert_eq!(kv.bucket, "settings");
    assert_eq!(kv.history, 3);
    assert_eq!(kv.max_value_size, 2048);
    assert_eq!(kv.max_bytes, 4096);
    assert_eq!(kv.max_age, Duration::from_secs(60));
    assert_eq!(kv.num_replicas, 2);
    assert_eq!(kv.description, "app settings");

    let object_store = ObjectStoreBucketConfigInput {
        bucket: "files".to_string(),
        storage: StorageKind::Memory,
        max_bytes: Some(8192),
        num_replicas: Some(1),
        description: None,
    }
    .into_async_nats();
    assert_eq!(object_store.bucket, "files");
    assert_eq!(
        object_store.storage,
        async_nats::jetstream::stream::StorageType::Memory
    );
    assert_eq!(object_store.max_bytes, 8192);
    assert_eq!(object_store.num_replicas, 1);
    assert_eq!(object_store.description, None);
}
