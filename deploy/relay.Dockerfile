FROM rust:1-slim AS build
WORKDIR /src
COPY . .
RUN cargo build --release --package openirl-relay

FROM debian:stable-slim
COPY --from=build /src/target/release/openirl-relay /usr/local/bin/openirl-relay
ENTRYPOINT ["/usr/local/bin/openirl-relay"]
