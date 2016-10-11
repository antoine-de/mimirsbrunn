## OSM fixtures

The sample OSM files to use are the *.osm.pbf ones.

For now, they contain a really small subset of real OSM data.

To add data to the OSM fixture :
* open the *.osm source file, with JOSM (File > Open)
* use the "Dowload Object" dialog (File > Dowload Object..) to select existing OSM object
* save the new source file (File > Save As...)
* transform the *.osm file into an *.osm.pbf file using osmconvert or osmosis

Do not forget to commit the usable *.osm.pbf file and the *.osm source file to ease the updates.

Note that if you modify the OSM data in JOSM you will need te remove the additions of the JOSM file-format in order to get a valid *.osm.pbf file.

See http://wiki.openstreetmap.org/wiki/JOSM_file_format to learn more.

## BANO fixtures

The BANO file-format is specified at http://bano.openstreetmap.fr/data/lisezmoi-bano.txt
