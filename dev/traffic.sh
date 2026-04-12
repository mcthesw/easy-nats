#!/usr/bin/env bash
# traffic.sh — Generate continuous NATS traffic for live testing
# Usage: bash dev/traffic.sh
# Stop with Ctrl+C

set -euo pipefail

SERVER="nats://localhost:4223"
NATS="nats --server=$SERVER"

COUNTER=0

echo "🔄 Generating traffic on $SERVER (Ctrl+C to stop)"
echo ""

while true; do
  COUNTER=$((COUNTER + 1))
  TS=$(date -Iseconds)

  # ── Orders: new order every 3s ──
  $NATS pub orders.created "{\"id\":$((1000+COUNTER)),\"item\":\"product-$((RANDOM%50))\",\"qty\":$((RANDOM%100+1)),\"ts\":\"$TS\"}" 2>/dev/null
  
  # ── Logs: random level every cycle ──
  LEVELS=("info" "info" "info" "warn" "error")  # weighted towards info
  LVL=${LEVELS[$((RANDOM % ${#LEVELS[@]}))]}
  $NATS pub "logs.$LVL" "{\"level\":\"$LVL\",\"msg\":\"Auto log #$COUNTER\",\"component\":\"traffic-gen\",\"ts\":\"$TS\"}" 2>/dev/null

  # ── Events: user activity ──
  USERS=("alice" "bob" "charlie" "diana")
  USR=${USERS[$((RANDOM % ${#USERS[@]}))]}
  ACTIONS=("login" "logout" "page_view" "click")
  ACT=${ACTIONS[$((RANDOM % ${#ACTIONS[@]}))]}
  $NATS pub "events.user.$ACT" "{\"user\":\"$USR\",\"action\":\"$ACT\",\"ts\":\"$TS\"}" 2>/dev/null

  # ── Core NATS (non-JetStream) subjects for pub/sub testing ──
  $NATS pub "demo.heartbeat" "{\"seq\":$COUNTER,\"ts\":\"$TS\"}" 2>/dev/null
  $NATS pub "demo.metrics" "{\"cpu\":$((RANDOM%100)),\"mem\":$((RANDOM%100)),\"seq\":$COUNTER}" 2>/dev/null

  # Print progress every 10 iterations
  if (( COUNTER % 10 == 0 )); then
    echo "  Published $COUNTER rounds ($((COUNTER * 5)) messages total)"
  fi

  sleep 3
done
