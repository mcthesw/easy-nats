#!/bin/sh
# traffic-inside.sh — generate continuous traffic, run inside nats-box
S="nats://nats-open:4222"
N="nats --server=$S"
C=0

echo "Generating traffic on $S (runs forever)..."

while true; do
  C=$((C + 1))

  # Orders
  $N pub orders.created "{\"id\":$((1000+C)),\"item\":\"product-$((C%50))\",\"qty\":$((C%100+1))}" 2>/dev/null

  # Logs (rotate levels)
  case $((C % 5)) in
    0) LVL=error ;;
    1) LVL=warn ;;
    *) LVL=info ;;
  esac
  $N pub "logs.$LVL" "{\"level\":\"$LVL\",\"msg\":\"Auto #$C\",\"src\":\"traffic\"}" 2>/dev/null

  # Events
  case $((C % 4)) in
    0) USR=alice; ACT=login ;;
    1) USR=bob; ACT=page_view ;;
    2) USR=charlie; ACT=click ;;
    3) USR=diana; ACT=logout ;;
  esac
  $N pub "events.user.$ACT" "{\"user\":\"$USR\",\"action\":\"$ACT\"}" 2>/dev/null

  # Core pub/sub (non-JetStream)
  $N pub "demo.heartbeat" "{\"seq\":$C}" 2>/dev/null
  $N pub "demo.metrics" "{\"cpu\":$((C%100)),\"mem\":$((C%80+20)),\"seq\":$C}" 2>/dev/null

  if [ $((C % 10)) -eq 0 ]; then
    echo "  [$C] $((C * 5)) messages published"
  fi

  sleep 3
done
