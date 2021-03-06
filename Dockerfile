FROM clux/muslrust:stable AS builder
WORKDIR /app
RUN mkdir src
COPY Cargo.toml ./
COPY src ./src/
COPY sqlx-data.json ./
ENV SQLX_OFFLINE true
RUN cargo build --release

FROM emerzon/alpine-mimalloc:latest
RUN apk add -q --progress --update --no-cache ca-certificates ffmpeg python3 && rm -rf /var/cache/apk/*
RUN ln -s /usr/bin/python3 /usr/bin/python && wget -q https://yt-dl.org/downloads/latest/youtube-dl -O /usr/local/bin/youtube-dl && chmod a+rx /usr/local/bin/youtube-dl
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/bowot /usr/local/bin
ADD sounds ./sounds
ENTRYPOINT ["bowot"]
