FROM node:14 AS ui-build

WORKDIR /usr/src/app
COPY . .
RUN --mount=type=cache,target=/usr/src/app/ui/node_modules \
    --mount=type=cache,target=/usr/src/app/ui/dist \
    cd ui \
    && npm install \
    && npm run build \
    && cp -r /usr/src/app/ui/dist /usr/src/ui-build

# ---
FROM rust:1.81.0-bullseye AS build

WORKDIR /usr/src/app

COPY --from=ui-build /usr/src/ui-build /usr/src/app/ui/dist
COPY . .

RUN --mount=type=cache,target=/usr/local/rustup \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build --release \
    && mv /usr/src/app/target/release/waterwheel /usr/bin

# ---
FROM debian:bullseye-slim

COPY --from=build /usr/bin/waterwheel /usr/bin

ENTRYPOINT ["/usr/bin/waterwheel"]
CMD ["--help"]
