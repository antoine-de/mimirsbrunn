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

COPY ./docker ./docker
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./build.rs ./build.rs
COPY ./config ./config
COPY ./benches ./benches
COPY ./tests ./tests
COPY ./libs ./libs
COPY ./src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/home/mimirsbrunn/target  \
    cargo build --profile production --bins --locked --features db-storage

# Extract binaries from build cache
RUN mkdir bin
RUN --mount=type=cache,target=/home/mimirsbrunn/target \
    cp target/production/osm2mimir                        \
       target/production/cosmogony2mimir                  \
       target/production/bano2mimir                       \
       target/production/openaddresses2mimir              \
       target/production/ctlmimir                         \
       bin/


ARG DEBIAN_VERSION

FROM debian:${DEBIAN_VERSION}-slim

WORKDIR /srv

ENV DEBIAN_FRONTEND noninteractive

ARG DEBIAN_VERSION

RUN if [ "${DEBIAN_VERSION}" = "buster" ]; then \
  apt-get update \
    && apt-get install -y libcurl4 sqlite3 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*; \
elif [ "${DEBIAN_VERSION}" = "stretch" ]; then \
  apt-get update \
    && apt-get install -y libcurl3 sqlite3 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*; \
else \
  echo "Unsupported debian version '$DEBIAN_VERSION'"; \
fi

COPY config /etc/mimirsbrunn
COPY docker/run_with_default_config.sh .
COPY --from=builder /home/mimirsbrunn/bin/* /usr/bin

ENTRYPOINT [ "./run_with_default_config.sh" ]
