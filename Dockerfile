FROM rust:slim-bookworm AS rust

RUN cargo install cargo-chef
RUN cargo install wasm-pack
RUN rustup target add wasm32-unknown-unknown
WORKDIR app

FROM rust AS plan
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust AS build-web
COPY . .
COPY --from=build-css /app/dist/style ./dist/style
RUN wasm-pack build --target web --release --out-dir dist/client client

FROM rust AS build-server
COPY --from=plan /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
COPY --from=build-web /app/dist ./dist
RUN cargo build --release --bin server

FROM debian:bookworm-slim AS runtime
WORKDIR app
COPY --from=build-server /app/target/release/server ./server
COPY --from=build-server /app/dist ./dist

EXPOSE 8080
CMD ["./server"]
