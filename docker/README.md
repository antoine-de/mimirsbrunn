# Docker

If you want to publish it on docker hub (to test it on docker_mimir for example), use:

```
docker login --username=YOUR_USERNAME # if not done already
docker build -f docker/Dockerfile_import . -t USERNAME/REPOSITORY:TAG
docker push USERNAME/REPOSITORY
```

## test with docker_mimir

Like said above, you can use the newly pushed image in `docker_mimir` repository for example. To do so, you'll need to update the `mimir` image in the `docker-compose.yml` file:

```yml
mimir:
  image: navitia/mimirsbrunn # change this line to USERNAME/REPOSITORY:TAG in your case!
```
