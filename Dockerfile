#Based of https://github.com/clux/muslrust
FROM rust:1.46 AS builder

WORKDIR /bowot
RUN mkdir src
COPY Cargo.toml ./
COPY src ./src/
RUN rustup show
RUN cargo install --path . --verbose

FROM debian:buster-slim
WORKDIR /bowot
RUN apt-get update && apt-get install -y libssl-dev pkg-config libsodium-dev libopus-dev ca-certificates
COPY --from=builder /usr/local/cargo/bin/bowot .
CMD [ "./bowot" ] 