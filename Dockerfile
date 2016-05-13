FROM debian:8

WORKDIR /srv

ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update
RUN apt-get install -y libcurl3
RUN apt-get clean
RUN rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

#you will need to update .dockerignore if you set this to another value
ARG SKOJIG_BIN=target/release/skojig

COPY $SKOJIG_BIN /srv/skojig

EXPOSE 4000
ENV ES_SKOJIG http://localhost:9200/munin
ENV RUST_LOG=debug,hyper=info

CMD /srv/skojig -b 0.0.0.0:4000

