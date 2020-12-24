FROM clux/muslrust:stable as planner
WORKDIR /app
RUN cargo install cargo-chef 
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM clux/muslrust:stable as cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM clux/muslrust:stable as builder
WORKDIR /app
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
RUN cargo build --release

FROM alpine:latest
RUN apk add -q --progress --update --no-cache ca-certificates ffmpeg python3 && rm -rf /var/cache/apk/*
RUN ln -s /usr/bin/python3 /usr/bin/python && wget -q https://yt-dl.org/downloads/latest/youtube-dl -O /usr/local/bin/youtube-dl && chmod a+rx /usr/local/bin/youtube-dl
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/bowot /usr/local/bin
ENTRYPOINT ["bowot"]