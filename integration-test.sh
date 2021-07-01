#!/bin/bash

elasticsearch_version="7.13.0"
elasticsearch_name="elasticsearch"

while true; do
    read -p "About to stop and remove docker container '$elasticsearch_name', and restart... Continue or Abort > " ca
    case $ca in
        [Cc]* ) docker stop elasticsearch; docker rm elasticsearch; break;;
        [Aa]* ) exit;;
        * ) echo "Please answer (c)ontinue or (a)bort.";;
    esac
done
docker run -p 9200:9200 -p 9300:9300 -e "discovery.type=single-node" --name $elasticsearch_name -d docker.elastic.co/elasticsearch/elasticsearch:$elasticsearch_version

./scripts/import2mimir.sh -c ./scripts/bretagne.rc

ELASTICSEARCH_TEST_URL="http://localhost:9200" cargo test --test integration 
