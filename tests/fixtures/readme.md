## OSM fixtures

Warning: Ideally, we'd like to have the XML file as a fixture: this would allow show diff when the fixture changes.
However, at the moment, the file `corse.osm.pbf` is about 26Mo, which is about 659Mo as an XML.
It would need to be seriously trimmed down before adding it to the `git` repository.

The sample OSM file to use is `corse.osm.pbf`.

To add data to the OSM fixture, you can transform into XML, add data, then transform back to PBF.
`osmosis` should allow you do these transformations back and forth.

```
docker run \
  --rm \
  --volume "${PWD}/tests/fixtures:/fixtures" \
  --workdir "/fixtures" \
  --entrypoint osmosis \
  yagajs/osmosis \
  --read-pbf file=/fixtures/corse.osm.pbf \
  --write-xml /fixtures/corse.osm
```

To edit the XML file, you can use JOSM.

Instead of using `osmosis`, you might be able to do everything from JOSM with [PBF plugin](https://wiki.openstreetmap.org/wiki/JOSM/Plugins/PBF).

Note with JOSM, you will to remove the additions of the JOSM file-format in order to get a valid `*.osm.pbf` file.

See http://wiki.openstreetmap.org/wiki/JOSM_file_format to learn more.

> NB: `osmconvert` can be used for the conversion, but some `name` tags may disappear on ways and relations

## Cosmogony fixtures

The cosmogony fixture `tests/fixtures/corse.jsonl.gz` is automatically created
from `tests/fixtures/corse.osm.pbf` when running `make test`.
