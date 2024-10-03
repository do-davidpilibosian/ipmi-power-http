FROM rust:1-slim-buster AS builder
WORKDIR /app
COPY . .
RUN \
  --mount=type=cache,target=/app/target/ \
  --mount=type=cache,target=/usr/local/cargo/registry/ \
  cargo build --release && \
  cp ./target/release/ipmi-power-http /

FROM debian:bookworm-slim AS final
RUN adduser \
  --disabled-password \
  --gecos "" \
  --home "/nonexistent" \
  --shell "/sbin/nologin" \
  --no-create-home \
  --uid "10001" \
  appuser
COPY --from=builder /ipmi-power-http /usr/local/bin
RUN chown appuser /usr/local/bin/ipmi-power-http
RUN mkdir /etc/ipmi-power-http/
RUN touch /etc/ipmi-power-http/config.yml
RUN apt update && apt install -y ipmitool
ENTRYPOINT ["ipmi-power-http", "--config-file", "/etc/ipmi-power-http/config.yml"]
EXPOSE 8080/tcp
