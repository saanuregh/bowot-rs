docker run \
    -v cargo-cache:/root/.cargo/registry \
    -v "$(pwd)":/home/rust/src \
    -e SQLX_OFFLINE=true \
    -u $(id -u ${USER}):$(id -g ${USER}) \
    --rm -it messense/rust-musl-cross:armv7-musleabihf \
    cargo build --release;