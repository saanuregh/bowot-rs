FROM rust:1.46 AS builder

WORKDIR /bowot
RUN mkdir src
COPY Cargo.toml ./
COPY src ./src/
RUN rustup show
RUN cargo install --path .

FROM python:slim
WORKDIR /bowot
RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev pkg-config libsodium-dev libopus-dev ca-certificates curl ffmpeg && rm -rf /var/cache/apt
RUN curl -L https://yt-dl.org/downloads/latest/youtube-dl -o /usr/local/bin/youtube-dl && chmod a+rx /usr/local/bin/youtube-dl
COPY --from=builder /usr/local/cargo/bin/bowot .
CMD [ "./bowot" ] 