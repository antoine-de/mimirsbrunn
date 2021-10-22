ARG RUST_VERSION="1.55"
ARG DEBIAN_VERSION="buster"

FROM rust:${RUST_VERSION}-${DEBIAN_VERSION} as builder

WORKDIR /home

ENV DEBIAN_FRONTEND noninteractive

ARG DEBIAN_VERSION

RUN if [ "${DEBIAN_VERSION}" = "buster" ]; then \
  apt-get update \
    && apt-get install -y make libssl-dev git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*; \
elif [ "${DEBIAN_VERSION}" = "stretch" ]; then \
  apt-get update \
    && apt-get install -y make libssl1.0-dev git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*; \
else \
  echo "Unsupported debian version '$DEBIAN_VERSION'"; \
fi

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
    cargo build --release --bin bragi --locked --features db-storage

# Extract binary from build cache
RUN mkdir bin
RUN --mount=type=cache,target=/home/mimirsbrunn/target \
    cp target/release/bragi bin/


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

COPY --from=builder /home/mimirsbrunn/bin/bragi /usr/bin/bragi
COPY --from=builder /home/mimirsbrunn/config /etc/bragi

EXPOSE 4000

ENV RUST_LOG                    info,hyper=info
ENV BRAGI_ELASTICSEARCH_URL     http://localhost:9200/munin
ENV BRAGI_SERVICE_HOST          0.0.0.0
ENV BRAGI_SERVICE_PORT          4000

CMD ["/usr/bin/bragi", "--config-dir", "/etc/bragi", "run"]
