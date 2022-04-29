ARG RUST_VERSION="1.60"
ARG DEBIAN_VERSION="buster"

FROM rust:${RUST_VERSION}-slim-${DEBIAN_VERSION} as builder

WORKDIR /home

ENV DEBIAN_FRONTEND noninteractive

ARG DEBIAN_VERSION

RUN apt-get update \
    && apt-get install -y pkg-config make libsqlite3-dev libssl-dev git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*;

RUN USER=root cargo new mimirsbrunn

WORKDIR /home/mimirsbrunn

COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./src ./src
COPY ./libs ./libs
COPY ./config ./config
COPY ./docker ./docker
COPY ./build.rs ./build.rs
COPY ./tests ./tests
COPY ./benches ./benches

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/home/mimirsbrunn/target  \
    cargo build --profile production --bin bragi --locked --features db-storage

# Extract binary from build cache
RUN mkdir bin
RUN --mount=type=cache,target=/home/mimirsbrunn/target \
    cp target/production/bragi bin/


ARG DEBIAN_VERSION

FROM debian:${DEBIAN_VERSION}-slim

WORKDIR /srv

ENV DEBIAN_FRONTEND noninteractive

ARG DEBIAN_VERSION

RUN if [ "${DEBIAN_VERSION}" = "buster" ]; then \
  apt-get update \
    && apt-get install -y curl libcurl4 sqlite3 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*; \
elif [ "${DEBIAN_VERSION}" = "stretch" ]; then \
  apt-get update \
    && apt-get install -y curl libcurl3 sqlite3 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*; \
else \
  echo "Unsupported debian version '$DEBIAN_VERSION'"; \
fi

COPY config /etc/bragi
COPY docker/run_with_default_config.sh .
COPY --from=builder /home/mimirsbrunn/bin/bragi /usr/bin/bragi

EXPOSE 4000

ENV CONFIG_DIR  /etc/bragi
ENV RUST_LOG    info,hyper=info

ENTRYPOINT [ "./run_with_default_config.sh", "bragi" ]
