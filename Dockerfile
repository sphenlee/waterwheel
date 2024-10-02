FROM rust:1.81.0-bullseye AS build

WORKDIR /usr/src/app

COPY . .
RUN --mount=type=cache,target=/usr/local/rustup \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build --release \
    && mv /usr/src/app/target/release/waterwheel /usr/bin


FROM debian:bullseye-slim

COPY --from=build /usr/bin/waterwheel /usr/bin

ENTRYPOINT ["/usr/bin/waterwheel"]
CMD ["--help"]
