Elasticsearch
=============

  * [Gathering Fields](#gathering-fields)
    * [Administrative Region](#administrative-region)
    * [Address](#address)
    * [Street](#street)
    * [POI](#point-of-interest)
    * [Stop](#stop)
  * [Templates](#templates)

This document describes the process of configuring Elasticsearch.

We can picture Elasticsearch as a black box, where we store JSON documents. These documents are of
different kinds, and depend on our business. Since we deal with geospatial data, and Navitia in
particular works with public transportations, the types of documents we store are:

* administrative regions: 
* addresses: 
* streets
* point of interests (POIs)
* stops (Public Transportations)

We first submit configuration files to Elasticsearch to describe how we want each document type to
be handled. These are so called component templates, and index templates, which include:
* settings: how do we want the text to be handled? do we want to use synonyms, lowercase, use stems,…
* mappings: how each type field of each type of document listed above is handled.

When the documents are indexed according to our settings and mappings, we can then query
Elasticsearch, and play with lots of parameters to push the ranking of documents up or down

To configure Elasticsearch, we'll first address settings, and mappings

# Gathering Fields

We'll construct a table with all the fields, for each type of document. The source of information is
the document, which is a rust structure serialized to JSON. When building this resource, be sure to
exclude what would be skipped (marked as `skip`) by the serializer.

## [Administrative Region](/libs/places/src/admin.rs)

<table>
<colgroup>
<col style="width: 19%" />
<col style="width: 21%" />
<col style="width: 58%" />
</colgroup>
<thead>
<tr class="header">
<th>field</th>
<th>type</th>
<th>description</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>administrative_regions</td>
<td>Vec&lt;Arc<Admin>&gt;</td>
<td>A list of parent administrative regions</td>
</tr>
<tr class="even">
<td>approx_coord</td>
<td>Option<Geometry></td>
<td>Coordinates of (the center??) of the region, similar to coord Given in lat lon</td>
</tr>
<tr class="odd">
<td>bbox</td>
<td>Option&lt;Rect<f64>&gt;</td>
<td>Bounding Box</td>
</tr>
<tr class="even">
<td>boundary</td>
<td>Option&lt;MultiPolygon<f64>&gt;</td>
<td>Describes the shape of the admin region</td>
</tr>
<tr class="odd">
<td>codes</td>
<td>BTreeMap&lt;String, String&gt;</td>
<td>Some codes used in OSM, like ISO3166, ref:nuts, wikidata</td>
</tr>
<tr class="even">
<td>context</td>
<td>Option<Context></td>
<td>Used for debugging</td>
</tr>
<tr class="odd">
<td>coord</td>
<td>Coord</td>
<td>Coordinates of the region</td>
</tr>
<tr class="even">
<td>country_codes</td>
<td>Vec<String></td>
<td>Country Codes</td>
</tr>
<tr class="odd">
<td>id</td>
<td>String</td>
<td>Unique id created by cosmogony</td>
</tr>
<tr class="even">
<td>insee</td>
<td>String</td>
<td>A code used to identify regions in France. From <a href="https://wiki.openstreetmap.org/wiki/Key:ref:INSEE?uselang=en">OSM</a></td>
</tr>
<tr class="odd">
<td>label</td>
<td>String</td>
<td>??</td>
</tr>
<tr class="even">
<td>labels</td>
<td>I18nProperties</td>
<td>??</td>
</tr>
<tr class="odd">
<td>level</td>
<td>u32</td>
<td>Position of the region in the admin hierarchy</td>
</tr>
<tr class="even">
<td>name</td>
<td>String</td>
<td>Name</td>
</tr>
<tr class="odd">
<td>names</td>
<td>I18nProperties</td>
<td>Name, but internationalized, eg name:en, name:ru, name:es</td>
</tr>
<tr class="even">
<td>parent_id</td>
<td>Option<String></td>
<td>id of the parent admin region (or none if root)</td>
</tr>
<tr class="odd">
<td>weight</td>
<td>f64</td>
<td>A number associated with the population in that region</td>
</tr>
<tr class="even">
<td>zip_codes</td>
<td>Vec<String></td>
<td>Zip codes (can be more than one)</td>
</tr>
<tr class="odd">
<td>zone_type</td>
<td>Option<ZoneType></td>
<td>Describes the type, eg city, suburb, country,…</td>
</tr>
</tbody>
</table>

## [Address](/libs/places/src/addr.rs

Addresses, compared to administrative regions, have very little unique fields, just house number and
street:

<table>
<colgroup>
<col style="width: 18%" />
<col style="width: 22%" />
<col style="width: 58%" />
</colgroup>
<thead>
<tr class="header">
<th>field</th>
<th>type</th>
<th>description</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>approx_coord</td>
<td>Option<Geometry></td>
<td></td>
</tr>
<tr class="even">
<td>context</td>
<td>Option<Context></td>
<td></td>
</tr>
<tr class="odd">
<td>coord</td>
<td>Coord</td>
<td></td>
</tr>
<tr class="even">
<td>country_codes</td>
<td>Vec<String></td>
<td></td>
</tr>
<tr class="odd">
<td>house_number</td>
<td>String</td>
<td>Identifier in the street</td>
</tr>
<tr class="even">
<td>id</td>
<td>String</td>
<td>Unique identifier</td>
</tr>
<tr class="odd">
<td>label</td>
<td>String</td>
<td></td>
</tr>
<tr class="even">
<td>name</td>
<td>String</td>
<td></td>
</tr>
<tr class="odd">
<td>street</td>
<td>Street</td>
<td>Reference to the street the address belongs to.</td>
</tr>
<tr class="even">
<td>weight</td>
<td>f64</td>
<td></td>
</tr>
<tr class="odd">
<td>zip_codes</td>
<td>Vec<String></td>
<td></td>
</tr>
</tbody>
</table>

## [Street](/libs/places/src/street.rs)

No particular fields for streets:

<table style="width:81%;">
<colgroup>
<col style="width: 34%" />
<col style="width: 26%" />
<col style="width: 19%" />
</colgroup>
<thead>
<tr class="header">
<th>field</th>
<th>type</th>
<th>description</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>administrative_regions</td>
<td>Vec&lt;Arc<Admin>&gt;</td>
<td></td>
</tr>
<tr class="even">
<td>approx_coord</td>
<td>Option<Geometry></td>
<td></td>
</tr>
<tr class="odd">
<td>context</td>
<td>Option<Context></td>
<td></td>
</tr>
<tr class="even">
<td>coord</td>
<td>Coord</td>
<td></td>
</tr>
<tr class="odd">
<td>country_codes</td>
<td>Vec<String></td>
<td></td>
</tr>
<tr class="even">
<td>id</td>
<td>String</td>
<td></td>
</tr>
<tr class="odd">
<td>label</td>
<td>String</td>
<td></td>
</tr>
<tr class="even">
<td>name</td>
<td>String</td>
<td></td>
</tr>
<tr class="odd">
<td>weight</td>
<td>f64</td>
<td></td>
</tr>
<tr class="even">
<td>zip_codes</td>
<td>Vec<String></td>
<td></td>
</tr>
</tbody>
</table>

## [Point of Interest](/libs/places/src/poi.rs)

<table>
<colgroup>
<col style="width: 28%" />
<col style="width: 31%" />
<col style="width: 40%" />
</colgroup>
<thead>
<tr class="header">
<th>field</th>
<th>type</th>
<th>description</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>address</td>
<td>Option
<Address></td>
<td>Address associated with that POI Can be an address or a street</td>
</tr>
<tr class="even">
<td>administrative_regions</td>
<td>Vec&lt;Arc<Admin>&gt;</td>
<td></td>
</tr>
<tr class="odd">
<td>approx_coord</td>
<td>Option<Geometry></td>
<td></td>
</tr>
<tr class="even">
<td>context</td>
<td>Option<Context></td>
<td></td>
</tr>
<tr class="odd">
<td>coord</td>
<td>Coord</td>
<td></td>
</tr>
<tr class="even">
<td>country_codes</td>
<td>Vec<String></td>
<td></td>
</tr>
<tr class="odd">
<td>id</td>
<td>String</td>
<td></td>
</tr>
<tr class="even">
<td>label</td>
<td>String</td>
<td></td>
</tr>
<tr class="odd">
<td>labels</td>
<td>I18nProperties</td>
<td></td>
</tr>
<tr class="even">
<td>name</td>
<td>String</td>
<td></td>
</tr>
<tr class="odd">
<td>names</td>
<td>I18nProperties</td>
<td></td>
</tr>
<tr class="even">
<td>poi_type</td>
<td>PoiType</td>
<td>id / name references in NTFS</td>
</tr>
<tr class="odd">
<td>properties</td>
<td>BTreeMap&lt;String, String&gt;</td>
<td></td>
</tr>
<tr class="even">
<td>weight</td>
<td>f64</td>
<td></td>
</tr>
<tr class="odd">
<td>zip_codes</td>
<td>Vec<String></td>
<td></td>
</tr>
</tbody>
</table>

## [Stop](/libs/places/src/stop.rs) (Public Transportations)

<table>
<colgroup>
<col style="width: 24%" />
<col style="width: 26%" />
<col style="width: 48%" />
</colgroup>
<thead>
<tr class="header">
<th>field</th>
<th>type</th>
<th>description</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>administrative_regions</td>
<td>Vec&lt;Arc<Admin>&gt;</td>
<td></td>
</tr>
<tr class="even">
<td>approx_coord</td>
<td>Option<Geometry></td>
<td></td>
</tr>
<tr class="odd">
<td>codes</td>
<td>BTreeMap&lt;String, String&gt;</td>
<td></td>
</tr>
<tr class="even">
<td>comments</td>
<td>Vec<Comment></td>
<td></td>
</tr>
<tr class="odd">
<td>commercial_modes</td>
<td>Vec<CommercialMode></td>
<td></td>
</tr>
<tr class="even">
<td>context</td>
<td>Option<Context></td>
<td></td>
</tr>
<tr class="odd">
<td>coord</td>
<td>Coord</td>
<td></td>
</tr>
<tr class="even">
<td>country_codes</td>
<td>Vec<String></td>
<td></td>
</tr>
<tr class="odd">
<td>coverages</td>
<td>Vec<String></td>
<td></td>
</tr>
<tr class="even">
<td>feed_publishers</td>
<td>Vec<FeedPublisher></td>
<td></td>
</tr>
<tr class="odd">
<td>id</td>
<td>String</td>
<td></td>
</tr>
<tr class="even">
<td>label</td>
<td>String</td>
<td></td>
</tr>
<tr class="odd">
<td>lines</td>
<td>Vec<Line></td>
<td></td>
</tr>
<tr class="even">
<td>name</td>
<td>String</td>
<td></td>
</tr>
<tr class="odd">
<td>physical_modes</td>
<td>Vec<PhysicalMode></td>
<td></td>
</tr>
<tr class="even">
<td>properties</td>
<td>BTreeMap&lt;String, String&gt;</td>
<td></td>
</tr>
<tr class="odd">
<td>timezone</td>
<td>String</td>
<td></td>
</tr>
<tr class="even">
<td>weight</td>
<td>f64</td>
<td>The weight depends on the number of lines, and other parameters.</td>
</tr>
<tr class="odd">
<td>zip_codes</td>
<td>Vec<String></td>
<td></td>
</tr>
</tbody>
</table>

# Templates

When we combine together all the fields from the previous documents, we obtain the following table,
which shows all the fields in use, and by what type of document.

<table>
<colgroup>
<col style="width: 17%" />
<col style="width: 19%" />
<col style="width: 41%" />
<col style="width: 4%" />
<col style="width: 4%" />
<col style="width: 4%" />
<col style="width: 4%" />
<col style="width: 4%" />
</colgroup>
<thead>
<tr class="header">
<th>field</th>
<th>type</th>
<th>description</th>
<th>adm</th>
<th>add</th>
<th>poi</th>
<th>stp</th>
<th>str</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>address</td>
<td>Option
<Address></td>
<td>Address associated with that POI</td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>administrative_regions</td>
<td>Vec&lt;Arc<Admin>&gt;</td>
<td>A list of parent administrative regions</td>
<td>✓</td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
</tr>
<tr class="odd">
<td>approx_coord</td>
<td>Option<Geometry></td>
<td>Coordinates of the object, similar to coord</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
</tr>
<tr class="even">
<td>bbox</td>
<td>Option&lt;Rect<f64>&gt;</td>
<td>Bounding Box</td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>boundary</td>
<td>Option&lt;MultiPolygon<f64>&gt;</td>
<td>Describes the shape of the admin region</td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>codes</td>
<td>BTreeMap&lt;String, String&gt;</td>
<td>Some codes used in OSM, like ISO3166, ref:nuts, wikidata</td>
<td>✓</td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
</tr>
<tr class="odd">
<td>comments</td>
<td>Vec<Comment></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
</tr>
<tr class="even">
<td>commercial_modes</td>
<td>Vec<CommercialMode></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
</tr>
<tr class="odd">
<td>context</td>
<td>Option&lt;Conte✓t&gt;</td>
<td>Used to return information (debugging)</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
</tr>
<tr class="even">
<td>coord</td>
<td>Coord</td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
</tr>
<tr class="odd">
<td>country_codes</td>
<td>Vec<String></td>
<td>Country Codes</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
</tr>
<tr class="even">
<td>coverages</td>
<td>Vec<String></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
</tr>
<tr class="odd">
<td>feed_publishers</td>
<td>Vec<FeedPublisher></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
</tr>
<tr class="even">
<td>house_number</td>
<td>String</td>
<td>Identifier in the street</td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>id</td>
<td>String</td>
<td>Unique identifier</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
</tr>
<tr class="even">
<td>insee</td>
<td>String</td>
<td>A code used to identify regions in France.</td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>label</td>
<td>String</td>
<td>??</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
</tr>
<tr class="even">
<td>labels</td>
<td>I18nProperties</td>
<td>??</td>
<td>✓</td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>level</td>
<td>u32</td>
<td>Position of the region in the admin hierarchy</td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>lines</td>
<td>Vec<Line></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
</tr>
<tr class="odd">
<td>name</td>
<td>String</td>
<td>Name</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
</tr>
<tr class="even">
<td>names</td>
<td>I18nProperties</td>
<td>Name, but internationalized, eg name:en, name:ru, name:es</td>
<td>✓</td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>parent_id</td>
<td>Option<String></td>
<td>id of the parent admin region (or none if root)</td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>physical_modes</td>
<td>Vec<PhysicalMode></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
</tr>
<tr class="odd">
<td>poi_type</td>
<td>PoiType</td>
<td>id / name references in NTFS</td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>properties</td>
<td>BTreeMap&lt;String, String&gt;</td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td>✓</td>
<td></td>
</tr>
<tr class="odd">
<td>street</td>
<td>Street</td>
<td>Reference to the street the address belongs to.</td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>timezone</td>
<td>String</td>
<td></td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
</tr>
<tr class="odd">
<td>weight</td>
<td>f64</td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
</tr>
<tr class="even">
<td>zip_codes</td>
<td>Vec<String></td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
</tr>
<tr class="odd">
<td>zone_type</td>
<td>Option<ZoneType></td>
<td>Describes the type, eg city, suburb, country,…</td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
</tbody>
</table>

We can extract from this table a list of fields that are (almost) common to all the documents. In
this table of common fields, we indicate what type is used for Elasticsearch, whether we should
index the field, and some comments.

<table style="width:100%;">
<colgroup>
<col style="width: 18%" />
<col style="width: 15%" />
<col style="width: 4%" />
<col style="width: 4%" />
<col style="width: 4%" />
<col style="width: 4%" />
<col style="width: 4%" />
<col style="width: 11%" />
<col style="width: 5%" />
<col style="width: 27%" />
</colgroup>
<thead>
<tr class="header">
<th>field</th>
<th>type</th>
<th>adm</th>
<th>add</th>
<th>poi</th>
<th>stp</th>
<th>str</th>
<th>Elasticsearch</th>
<th>Index</th>
<th>Comment</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>administrative_regions</td>
<td><code>Vec&lt;Arc&lt;Admin&gt;&gt;</code></td>
<td>✓</td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td></td>
<td>✗</td>
<td>large object</td>
</tr>
<tr class="even">
<td>approx_coord</td>
<td><code>Option&lt;Geometry&gt;</code></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>??</td>
<td>??</td>
<td>Improved coord in Elasticsearch may render <code>approx_coord</code> obsolete</td>
</tr>
<tr class="odd">
<td>context</td>
<td><code>Option&lt;Context&gt;</code></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td></td>
<td>✗</td>
<td>Output</td>
</tr>
<tr class="even">
<td>coord</td>
<td><code>Coord</code></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>geo_point</td>
<td>✓</td>
<td>Index for reverse API</td>
</tr>
<tr class="odd">
<td>country_codes</td>
<td><code>Vec&lt;String&gt;</code></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>??</td>
<td>??</td>
<td>Are we searching with these ?</td>
</tr>
<tr class="even">
<td>id</td>
<td><code>String</code></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>keyword</td>
<td>✓</td>
<td>Index for features API</td>
</tr>
<tr class="odd">
<td>label</td>
<td><code>String</code></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>text</td>
<td>✓</td>
<td>??</td>
</tr>
<tr class="even">
<td>name</td>
<td><code>String</code></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>text</td>
<td>✓</td>
<td>copy to <code>full label</code></td>
</tr>
<tr class="odd">
<td>weight</td>
<td><code>f64</code></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>float</td>
<td>✗</td>
<td>used for ranking</td>
</tr>
<tr class="even">
<td>zip_codes</td>
<td><code>Vec&lt;String&gt;</code></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>text</td>
<td>??</td>
<td>copy to <code>full label</code></td>
</tr>
</tbody>
</table>

We use component templates and index templates.

Component templates must not depend from other component templates. So component templates must be
self-contained. This means that if you have, in a component template, a mapping which uses a
specific analyzer, which in turn uses a specific tokenizer for example, then the definitions of the
analyzer and the tokenizer must be present in that component template. This suggest that the
strategy used to breakdown templates into components should be to include small reusable components:

We'll have:

* **mimir-base**: a component which includes fields that are present in all types of documents, and
	that don't necessitate a specific tokenizer, analyzer, or filter.
* **mimir-text**: a component for indexing text based fields.
* **mimir-dynamic**: a component for dynamic templates.

This is of course facilitated by the fact that few, if any, field specific to a place need its own
analyzer.

All template names are prefixed with 'mimir-', so that it is easy to list them, or delete them with 
a regular expression.

The following table shows all the fields that are used in documents, what types they are, what
indexes they belong to, and possibly what template they belong to:

<table>
<colgroup>
<col style="width: 13%" />
<col style="width: 9%" />
<col style="width: 8%" />
<col style="width: 26%" />
<col style="width: 6%" />
<col style="width: 8%" />
<col style="width: 7%" />
<col style="width: 4%" />
<col style="width: 5%" />
<col style="width: 10%" />
</colgroup>
<tbody>
<tr class="odd">
<td></td>
<td>type</td>
<td>indexed</td>
<td>description</td>
<td>admin</td>
<td>address</td>
<td>street</td>
<td>poi</td>
<td>stop</td>
<td>component template</td>
</tr>
<tr class="even">
<td>admin_regions</td>
<td></td>
<td>✗</td>
<td>hierarchy of admin regions</td>
<td>✓</td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td></td>
</tr>
<tr class="odd">
<td>approx_coord</td>
<td>geo_shape</td>
<td>?</td>
<td>FIXME to be removed ?? duplicate of coord</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>mimir-base</td>
</tr>
<tr class="even">
<td>coord</td>
<td>geo_shape</td>
<td>✓</td>
<td>lat / lon coordinate</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>mimir-base</td>
</tr>
<tr class="odd">
<td>coverages</td>
<td>text</td>
<td>✗</td>
<td>names of datasets (FIXME ref)</td>
<td></td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
</tr>
<tr class="even">
<td>full_label</td>
<td>text</td>
<td></td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>mimir-text</td>
</tr>
<tr class="odd">
<td>house_number</td>
<td>text</td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>id</td>
<td>keyword</td>
<td>✗</td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>mimir-base</td>
</tr>
<tr class="odd">
<td>indexed_at</td>
<td>date</td>
<td></td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>mimir-base</td>
</tr>
<tr class="even">
<td>insee</td>
<td>keyword</td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>level</td>
<td>long</td>
<td>✗</td>
<td>admin level (FIXME ref)</td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>parent_id</td>
<td>keyword</td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>poi_type</td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>properties</td>
<td>flattened</td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td></td>
</tr>
<tr class="odd">
<td>street</td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>weight</td>
<td>double</td>
<td></td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>mimir-base</td>
</tr>
<tr class="odd">
<td>zip_codes</td>
<td>text</td>
<td></td>
<td></td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>✓</td>
<td>mimir-text</td>
</tr>
<tr class="even">
<td>zone_type</td>
<td>keyword</td>
<td></td>
<td></td>
<td>✓</td>
<td></td>
<td></td>
<td></td>
<td></td>
<td></td>
</tr>
</tbody>
</table>
