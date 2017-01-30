# docker file used in jenkins to build the project
FROM jimmycuadra/rust:latest

# we also need a docker client
 RUN apt-get update &&\
    apt-get install -y curl ssh git python &&\
    curl "https://get.docker.com/builds/Linux/x86_64/docker-1.13.0.tgz" > docker.tgz &&\
    tar -xvzf docker.tgz &&\
    mv docker/* /usr/bin &&\
    rm -rf docke* &&\
# and fpm to create the debian packages
    apt-get install -y ruby ruby-dev rubygems gcc make &&\
    gem install --no-ri --no-rdoc fpm &&\
# we cleanup the cache to reduce the image size
    rm -rf /var/lib/apt/lists/*
