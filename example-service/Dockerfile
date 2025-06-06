ARG RUST_VERSION=1.84.0
ARG APP_NAME=example-service
FROM rust:${RUST_VERSION}-slim-bullseye AS build
ARG APP_NAME
WORKDIR /app

RUN apt-get update && apt-get install -y protobuf-compiler

# Copy source files into the container
COPY src/ src/
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY build.rs build.rs
COPY proto/ proto/

# Build the app
RUN set -e && \
    cargo build --locked --release && \
    cp ./target/release/$APP_NAME /bin/server

FROM debian:bullseye-slim AS final

ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

COPY --from=build /bin/server /bin/

ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
EXPOSE 3001
HEALTHCHECK --interval=30s --timeout=30s --start-period=5s --retries=3 CMD [ "curl", "-fL", "http://localhost:3001/health" ]

CMD ["/bin/server"]
