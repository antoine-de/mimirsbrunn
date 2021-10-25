# Elasticsearch

## Templates

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
