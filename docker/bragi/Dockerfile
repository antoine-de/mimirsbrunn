ARG RUST_VERSION
ARG DEBIAN_VERSION

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

RUN echo "// dummy file" > src/lib.rs
RUN echo "fn main () { }" > ./build.rs

RUN mkdir -p libs/bragi/src
COPY ./libs/bragi/Cargo.toml ./libs/bragi/Cargo.toml
RUN echo "// dummy file" > ./libs/bragi/src/lib.rs

RUN mkdir -p libs/mimir/src
COPY ./libs/mimir/Cargo.toml ./libs/mimir/Cargo.toml
RUN echo "// dummy file" > ./libs/mimir/src/lib.rs

RUN mkdir -p libs/docker_wrapper/src
COPY ./libs/docker_wrapper/Cargo.toml ./libs/docker_wrapper/Cargo.toml
RUN echo "// dummy file" > ./libs/docker_wrapper/src/lib.rs

RUN mkdir -p libs/tools/src
COPY ./libs/tools/Cargo.toml ./libs/tools/Cargo.toml
RUN echo "// dummy file" > ./libs/tools/src/lib.rs

RUN mkdir -p tests
RUN echo "// dummy file" > ./tests/tests.rs

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src
COPY ./libs ./libs
COPY ./config ./config
COPY ./docker ./docker
COPY ./build.rs ./build.rs

RUN rm ./target/release/deps/libmimir*
RUN rm ./target/release/deps/libbragi*
RUN rm ./target/release/deps/mimirsbrunn*

RUN cargo build --release --features db-storage

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

COPY --from=builder /home/mimirsbrunn/target/release/bragi /usr/bin/bragi

EXPOSE 4000
ENV BRAGI_ES http://localhost:9200/munin
ENV RUST_LOG=info,hyper=info

CMD ["/usr/bin/bragi", "-b", "0.0.0.0:4000"]
