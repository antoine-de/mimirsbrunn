# Troubleshooting


### Multiple streets with the same name and admin in OSM

Some almost identical streets are difficult to distinguish in OSM, as
they are in the same admin and have the same name, but still
they are distinct ones.  
Ex: Rue Jean Jaurès in Lille (France)

_osm2mimir_ merges ways with same name and admin by default. This helps managing streets split by a place and many other cases.

It can also distinguish them if they are part of different `Relation:associatedStreet`, so
it's the way to go if one wants to have distinct entries: https://wiki.openstreetmap.org/wiki/Relation:associatedStreet
