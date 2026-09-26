# syntax=docker/dockerfile:1
# check=error=true

# For building the binary
FROM rust:1.98.1-slim-bookworm AS build
WORKDIR /build
RUN mkdir /data
COPY Cargo.toml Cargo.lock ./
COPY openapi ./openapi
COPY src ./src
RUN cargo build --release --locked

# For deployment
FROM gcr.io/distroless/cc-debian12:nonroot AS deploy
COPY --from=litestream/litestream:0.5.17 /usr/local/bin/litestream /usr/local/bin/
COPY --from=build /build/target/release/sqlite-cloudrun /usr/local/bin/
COPY --from=build --chown=65532:65532 /data /data
COPY litestream.yaml /etc/litestream.yml
ENV LITESTREAM_SOCKET=/tmp/litestream.sock

# The server stops the old revision, restores from GCS, then starts `litestream replicate` itself.
ENTRYPOINT ["/usr/local/bin/sqlite-cloudrun"]
