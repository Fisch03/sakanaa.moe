FROM rust:slim-bookworm AS rust

RUN cargo install cargo-chef
RUN cargo install wasm-pack
RUN rustup target add wasm32-unknown-unknown
WORKDIR app

FROM rust AS plan
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM google/dart AS sass

ARG DART_SASS_VERSION=1.97.2
ARG DART_SASS_TAR=dart-sass-${DART_SASS_VERSION}-linux-x64.tar.gz
ARG DART_SASS_URL=https://github.com/sass/dart-sass/releases/download/${DART_SASS_VERSION}/${DART_SASS_TAR}

ADD ${DART_SASS_URL} /opt/
RUN cd /opt/ && tar -xzf ${DART_SASS_TAR} && rm ${DART_SASS_TAR}

FROM sass AS build-css
WORKDIR app
COPY . .
# RUN /opt/dart-sass/sass --embed-sources --load-path common/style common/style:dist/style
RUN /opt/dart-sass/sass --embed-sources common/style:dist/style

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
