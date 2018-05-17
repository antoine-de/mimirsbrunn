FROM debian:8

WORKDIR /srv

ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update && apt-get install -y libcurl3 libgeos-c1 && apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

#you will need to update .dockerignore if you set this to another value
ARG BRAGI_BIN=target/release/bragi

COPY $BRAGI_BIN /srv/bragi

EXPOSE 4000
ENV BRAGI_ES http://localhost:9200/munin
ENV RUST_LOG=debug,hyper=info

CMD ["/srv/bragi", "-b", "0.0.0.0:4000"]
