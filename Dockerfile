FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/vpn-server1 /app/vpn-server1
EXPOSE 1080
CMD ["/app/vpn-server1"]
