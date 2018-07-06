#! /bin/bash
set -e
set -x

pip3 install docker-compose pipenv
git clone -b master https://github.com/QwantResearch/docker_mimir.git
mv ci/gitlab/*.yml docker_mimir/
cd docker_mimir && pipenv install --system --deploy
