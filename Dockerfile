FROM rust:1.48 as builder

WORKDIR /usr/src/lantern
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/lantern /usr/local/bin/lantern
CMD ["lantern"]