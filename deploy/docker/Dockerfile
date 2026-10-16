FROM rust:1.88-bookworm AS build
WORKDIR /src
COPY server/ server/
COPY cli/ cli/
COPY web/ web/
RUN cargo build --release --locked --manifest-path server/Cargo.toml \
    && cargo build --release --locked --manifest-path cli/Cargo.toml \
    && strip server/target/release/pocket-share-server cli/target/release/pshare

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /data && chown 10001:10001 /data
COPY --from=build /src/server/target/release/pocket-share-server /usr/local/bin/
COPY --from=build /src/cli/target/release/pshare /usr/local/bin/
COPY deploy/docker/config.toml /etc/pocketshare/config.toml
USER 10001:10001
VOLUME ["/data"]
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s CMD curl -fsS http://127.0.0.1:3000/health || exit 1
ENTRYPOINT ["pocket-share-server"]
CMD ["--config", "/etc/pocketshare/config.toml"]
