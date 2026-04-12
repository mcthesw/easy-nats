#!/bin/sh
# seed-inside.sh — run inside nats-box container
set -e

S="nats://nats-open:4222"
N="nats --server=$S"

echo "=== Seeding NATS ==="

# ── Consumers via JSON config ──
echo '── Creating Consumers ──'

echo '{"durable_name":"order-processor","filter_subject":"orders.created","ack_policy":"explicit","max_deliver":5,"deliver_policy":"all","replay_policy":"instant","max_ack_pending":1000}' > /tmp/c1.json
$N consumer add ORDERS order-processor --config /tmp/c1.json 2>/dev/null && echo "  ✓ order-processor" || echo "  (exists) order-processor"

echo '{"durable_name":"order-analytics","ack_policy":"all","deliver_policy":"last","replay_policy":"instant","max_ack_pending":1000}' > /tmp/c2.json
$N consumer add ORDERS order-analytics --config /tmp/c2.json 2>/dev/null && echo "  ✓ order-analytics" || echo "  (exists) order-analytics"

echo '{"durable_name":"event-sink","ack_policy":"explicit","deliver_policy":"new","replay_policy":"instant","max_ack_pending":1000}' > /tmp/c3.json
$N consumer add EVENTS event-sink --config /tmp/c3.json 2>/dev/null && echo "  ✓ event-sink" || echo "  (exists) event-sink"

echo '{"durable_name":"log-archiver","filter_subject":"logs.error","ack_policy":"none","deliver_policy":"all","replay_policy":"instant"}' > /tmp/c4.json
$N consumer add LOGS log-archiver --config /tmp/c4.json 2>/dev/null && echo "  ✓ log-archiver" || echo "  (exists) log-archiver"

# ── KV Buckets ──
echo ''
echo '── Creating KV Buckets ──'
$N kv add config --history=5 --storage=file --replicas=1 2>/dev/null && echo "  ✓ config" || echo "  (exists) config"
$N kv add sessions --history=1 --ttl=30m --storage=memory --replicas=1 2>/dev/null && echo "  ✓ sessions" || echo "  (exists) sessions"
$N kv add feature-flags --history=10 --storage=file --replicas=1 2>/dev/null && echo "  ✓ feature-flags" || echo "  (exists) feature-flags"

# ── Seed KV entries ──
echo ''
echo '── Seeding KV Entries ──'
$N kv put config app.name "Easy NATS"
$N kv put config app.version "0.1.0"
$N kv put config db.host "localhost"
$N kv put config db.port "5432"
$N kv put config cache.ttl "300"
echo "  ✓ config: 5 entries"

$N kv put sessions user-alice '{"user":"alice","role":"admin","login_at":"2026-04-12T10:00:00Z"}'
$N kv put sessions user-bob '{"user":"bob","role":"viewer","login_at":"2026-04-12T10:05:00Z"}'
echo "  ✓ sessions: 2 entries"

$N kv put feature-flags dark-mode "true"
$N kv put feature-flags beta-export "false"
$N kv put feature-flags max-connections "50"
$N kv put feature-flags dark-mode "false"
$N kv put feature-flags dark-mode "true"
echo "  ✓ feature-flags: 3 entries (with history)"

# ── Seed stream messages ──
echo ''
echo '── Publishing seed messages ──'
for i in 1 2 3 4 5; do
  $N pub orders.created "{\"id\":$i,\"item\":\"widget-$i\",\"qty\":$((i*10))}"
done
echo "  ✓ 5 messages → orders.created"

for i in 1 2 3; do
  $N pub orders.shipped "{\"id\":$i,\"carrier\":\"express\"}"
done
echo "  ✓ 3 messages → orders.shipped"

for level in info warn error; do
  for i in 1 2 3; do
    $N pub "logs.$level" "{\"level\":\"$level\",\"msg\":\"Test log #$i\"}"
  done
done
echo "  ✓ 9 messages → logs.{info,warn,error}"

$N pub events.user.login '{"user":"alice","ip":"10.0.0.1"}'
$N pub events.user.login '{"user":"bob","ip":"10.0.0.2"}'
$N pub events.order.placed '{"order_id":42,"total":99.99}'
echo "  ✓ 3 messages → events.*"

echo ''
echo '=== Seed complete ==='
