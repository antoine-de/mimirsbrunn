## OSM fixtures

The sample OSM files to use are the *.osm.pbf ones.

For now, they contain a really small subset of real OSM data.

To add data to the OSM fixture :
* open the *.osm source file, with JOSM (File > Open)
* use the "Dowload Object" dialog (File > Dowload Object..) to select existing OSM object
* save the new source file (File > Save As...)
* transform the *.osm file into an *.osm.pbf file using osmosis

Example of osmosis command line :
`osmosis --read-xml file="osm_fixture.osm" --write-pbf file="osm_fixture.osm.pbf"`

> NB: osmcnvert can be used for the conversion, but some *name* tags disapears on ways and relations

Do not forget to commit the usable *.osm.pbf file and the *.osm source file to ease the updates.

Note that if you modify the OSM data in JOSM you will need te remove the additions of the JOSM file-format in order to get a valid *.osm.pbf file.

See http://wiki.openstreetmap.org/wiki/JOSM_file_format to learn more.

## BANO fixtures

The BANO file-format is specified at http://bano.openstreetmap.fr/data/lisezmoi-bano.txt

## OSM test data
### Cities
type | name
--- | ---
relation | Vaux-le-Pénil
relation | Livry-sur-Seine  
relation | Melun*
relation | Melun (not a valid boundary)
relation | Le Coudray-Montceaux

> *: Melun is modified to bear multiple postcodes, including numbers and letters (CP77001;77000;77008;77003).

### POIs
The .osm file contains the following Data :

poi type | poi category | name | city
--- | --- | --- | ---
relation | amenity=parking | Parking | Le Coudray-Montceaux
relation | amenity=parking | Parking | no city provided
relation | amenity=prison | Centre de semi-liberté de Melun | Melun
way | amenity=townhall | Hôtel de Ville | no city provided
way | amenity=townhall | Hôtel de Ville | Melun
point | amenity=post_office | Le-Mée-sur-Seine Courtilleraies | no city provided
point | amenity=post_office | Melun Rp | Melun
