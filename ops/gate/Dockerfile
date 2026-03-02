FROM rust:1.90 as builder
WORKDIR /app
COPY . .
RUN cargo build -p ubl_gate --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /opt/ubl-core
COPY --from=builder /app/target/release/ubl_gate /usr/local/bin/ubl_gate
ENV UBL_GATE_BIND=0.0.0.0:4000
EXPOSE 4000
CMD ["ubl_gate"]
