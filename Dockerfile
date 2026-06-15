FROM rust:bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -rf src
COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y tmux iputils-ping curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
ARG TARGETARCH
RUN curl -fL "https://github.com/tsl0922/ttyd/releases/latest/download/ttyd.$([ "$TARGETARCH" = arm64 ] && echo aarch64 || echo x86_64)" \
    -o /usr/local/bin/ttyd && chmod +x /usr/local/bin/ttyd
COPY --from=builder /app/target/release/deadman-rs /usr/local/bin/deadman
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
# tmux: 内部ターミナルを xterm-256color に固定し italic/strikethrough を通す
RUN printf '%s\n' \
    'set -g default-terminal "xterm-256color"' \
    'set -ag terminal-overrides ",xterm-256color:Tc:sitm=\E[3m:ritm=\E[23m"' \
    > /root/.tmux.conf
EXPOSE 7681
ENTRYPOINT ["/entrypoint.sh"]
