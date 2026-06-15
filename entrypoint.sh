#!/bin/bash
set -e

SESSION="deadman"
CONFIG="${DEADMAN_CONFIG:-/etc/deadman/deadman.toml}"

# 既存セッションがあれば削除（コンテナ再起動時の考慮）
tmux kill-session -t "$SESSION" 2>/dev/null || true

# tmux セッション内で deadman を起動
tmux new-session -d -s "$SESSION" "deadman $CONFIG; exec bash"

echo "deadman started in tmux session: $SESSION"
echo "ttyd starting on port ${TTYD_PORT:-7681} (read-only)"

# ttyd を read-only モードで起動
# -R : read-only（閲覧者はキー入力できない）
# -p : ポート
# -t : ターミナルオプション（フォントサイズ等）
exec ttyd \
    --port "${TTYD_PORT:-7681}" \
    --readonly \
    ${TTYD_CREDENTIAL:+--credential "$TTYD_CREDENTIAL"} \
    --terminal-type xterm-256color \
    -t fontSize=14 \
    -t 'theme={"background":"#1a1a2e"}' \
    -t 'unicodeVersion=11' \
    tmux attach-session -t "$SESSION" -r
