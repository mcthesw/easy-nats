#!/usr/bin/env bash
# seed.sh — Create streams, consumers, and KV buckets on the open NATS server (port 4223, no auth)
# Usage: bash dev/seed.sh
# Requires: nats CLI (https://github.com/nats-io/natscli)

set -euo pipefail

SERVER="nats://localhost:4223"
NATS="nats --server=$SERVER"

echo "=== Seeding NATS (open server @ $SERVER) ==="

# ── Streams ──────────────────────────────────────────────
echo ""
echo "── Creating Streams ──"

$NATS stream add ORDERS \
  --subjects="orders.>" \
  --retention=limits --storage=file \
  --max-msgs=10000 --max-bytes=10485760 --max-age=1h \
  --replicas=1 --discard=old --defaults 2>/dev/null || true
echo "  ✓ ORDERS (subjects: orders.>)"

$NATS stream add EVENTS \
  --subjects="events.>" \
  --retention=interest --storage=memory \
  --max-msgs=5000 --max-bytes=5242880 --max-age=30m \
  --replicas=1 --discard=old --defaults 2>/dev/null || true
echo "  ✓ EVENTS (subjects: events.>)"

$NATS stream add LOGS \
  --subjects="logs.>" \
  --retention=limits --storage=file \
  --max-msgs=50000 --max-bytes=52428800 --max-age=24h \
  --replicas=1 --discard=old --defaults 2>/dev/null || true
echo "  ✓ LOGS (subjects: logs.>)"

# ── Consumers ────────────────────────────────────────────
echo ""
echo "── Creating Consumers ──"

$NATS consumer add ORDERS order-processor \
  --pull --durable=order-processor \
  --filter="orders.created" \
  --ack=explicit --max-deliver=5 --deliver=all \
  --defaults 2>/dev/null || true
echo "  ✓ ORDERS / order-processor (filter: orders.created)"

$NATS consumer add ORDERS order-analytics \
  --pull --durable=order-analytics \
  --filter="" \
  --ack=all --deliver=last \
  --defaults 2>/dev/null || true
echo "  ✓ ORDERS / order-analytics (all subjects)"

$NATS consumer add EVENTS event-sink \
  --pull --durable=event-sink \
  --ack=explicit --deliver=new \
  --defaults 2>/dev/null || true
echo "  ✓ EVENTS / event-sink"

$NATS consumer add LOGS log-archiver \
  --pull --durable=log-archiver \
  --filter="logs.error" \
  --ack=none --deliver=all \
  --defaults 2>/dev/null || true
echo "  ✓ LOGS / log-archiver (filter: logs.error)"

# ── KV Buckets ───────────────────────────────────────────
echo ""
echo "── Creating KV Buckets ──"

$NATS kv add config \
  --history=5 --ttl=0 --max-value-size=1048576 \
  --storage=file --replicas=1 2>/dev/null || true
echo "  ✓ config (history=5)"

$NATS kv add sessions \
  --history=1 --ttl=30m --max-value-size=65536 \
  --storage=memory --replicas=1 2>/dev/null || true
echo "  ✓ sessions (ttl=30m, memory)"

$NATS kv add feature-flags \
  --history=10 --ttl=0 --max-value-size=4096 \
  --storage=file --replicas=1 2>/dev/null || true
echo "  ✓ feature-flags (history=10)"

# ── Seed KV entries ──────────────────────────────────────
echo ""
echo "── Seeding KV Entries ──"

$NATS kv put config app.name "Easy NATS" 2>/dev/null
$NATS kv put config app.version "0.1.0" 2>/dev/null
$NATS kv put config db.host "localhost" 2>/dev/null
$NATS kv put config db.port "5432" 2>/dev/null
$NATS kv put config cache.ttl "300" 2>/dev/null
echo "  ✓ config: 5 entries"

$NATS kv put sessions user-alice '{"user":"alice","role":"admin","login_at":"2026-04-12T10:00:00Z"}' 2>/dev/null
$NATS kv put sessions user-bob '{"user":"bob","role":"viewer","login_at":"2026-04-12T10:05:00Z"}' 2>/dev/null
echo "  ✓ sessions: 2 entries"

$NATS kv put feature-flags dark-mode "true" 2>/dev/null
$NATS kv put feature-flags beta-export "false" 2>/dev/null
$NATS kv put feature-flags max-connections "50" 2>/dev/null
# Update a few times to generate history
$NATS kv put feature-flags dark-mode "false" 2>/dev/null
$NATS kv put feature-flags dark-mode "true" 2>/dev/null
echo "  ✓ feature-flags: 3 entries (with history)"

# ── Seed some stream messages ────────────────────────────
echo ""
echo "── Publishing seed messages ──"

for i in $(seq 1 5); do
  $NATS pub orders.created "{\"id\":$i,\"item\":\"widget-$i\",\"qty\":$((i * 10)),\"ts\":\"$(date -Iseconds)\"}" 2>/dev/null
done
echo "  ✓ 5 messages → orders.created"

for i in $(seq 1 3); do
  $NATS pub orders.shipped "{\"id\":$i,\"carrier\":\"express\",\"ts\":\"$(date -Iseconds)\"}" 2>/dev/null
done
echo "  ✓ 3 messages → orders.shipped"

for level in info warn error; do
  for i in $(seq 1 3); do
    $NATS pub "logs.$level" "{\"level\":\"$level\",\"msg\":\"Test log entry $i\",\"ts\":\"$(date -Iseconds)\"}" 2>/dev/null
  done
done
echo "  ✓ 9 messages → logs.{info,warn,error}"

$NATS pub events.user.login '{"user":"alice","ip":"10.0.0.1"}' 2>/dev/null
$NATS pub events.user.login '{"user":"bob","ip":"10.0.0.2"}' 2>/dev/null
$NATS pub events.order.placed '{"order_id":42,"total":99.99}' 2>/dev/null
echo "  ✓ 3 messages → events.*"

echo ""
echo "=== Seed complete ==="
echo ""
echo "Summary:"
echo "  Streams:   ORDERS, EVENTS, LOGS"
echo "  Consumers: order-processor, order-analytics, event-sink, log-archiver"
echo "  KV:        config (5 keys), sessions (2 keys), feature-flags (3 keys + history)"
