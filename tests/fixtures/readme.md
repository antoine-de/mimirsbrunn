## OSM fixtures

The sample OSM file to use is `osm_fixture.osm.pbf`.

It contains a small subset of real OSM data, and a few fake admins.

To add data to the OSM fixture :
* open the `*.osm` source file, with JOSM (File > Open)
* use the "Download Object" dialog (File > Download Object..) to select an existing OSM object
* save the new source file (File > Save As...)
* transform the `*.osm` file into an `*.osm.pbf` file using osmosis or JOSM pbf plugin

Example of osmosis command line :
`osmosis --read-xml file="osm_fixture.osm" --write-pbf file="osm_fixture.osm.pbf"`

> NB: osmconvert can be used for the conversion, but some `name` tags disapears on ways and relations

Do not forget to commit the usable `*.osm.pbf` file and the `*.osm` source file to ease the updates.

Note that if you modify the OSM data in JOSM you will need te remove the additions of the JOSM file-format in order to get a valid `*.osm.pbf` file.

See http://wiki.openstreetmap.org/wiki/JOSM_file_format to learn more.

### Content

####  Boundaries
type | name
--- | ---
relation | Le Coudray-Montceaux (city)
relation | Livry-sur-Seine (city)
relation | Melun (city). Modified (see below)
relation | Saint-Martin-d'Hères (city)
relation | Vaux-le-Pénil (city)
relation | Créteil (`arrondissement`). Not real data.
relation | Fausse Seine-et-Marne (`département`). Not real data.
relation | France hexagonale (country). Not real data.
relation | Melun (`canton`, a non administrive zone). Incomplete, not a valid boundary

Melun city has been modified to bear multiple postcodes, including numbers and letters (CP77001;77000;77008;77003).

The file also contains a few other incomplete relations.

#### POIs
The OSM file contains the following objects :

poi type | poi category | name | city
--- | --- | --- | ---
relation | amenity=parking | Parking | Le Coudray-Montceaux
relation | amenity=parking | Parking | no city provided
relation | amenity=prison | Centre de semi-liberté de Melun | Melun
way | amenity=townhall | Hôtel de Ville | no city provided
way | amenity=townhall | Hôtel de Ville | Melun
point | amenity=post_office | Le-Mée-sur-Seine Courtilleraies | no city provided
point | amenity=post_office | Melun Rp | Melun


## BANO fixtures

The BANO file-format is specified at http://bano.openstreetmap.fr/data/lisezmoi-bano.txt

## Cosmogony2mimir
