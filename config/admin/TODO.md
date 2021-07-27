# Notes about settings and mappings

## Settings

 * `"max_gram": 10`
 Was 20. Why not 20 (ex: german names)

 * No synonyms. Critical for addresses (bd av etc).

 * Analyzers
 	* No prefix without ellision. (Perhaps it was unused with latest versions ?)
 	* "autocomplete" seems equivalent to "prefix_elision"
 	* "autocomplete_search" seems equivalent to "word_elision"
 	* No "ngram" tokenizer for fuzzy search ?

 * What's the point of the "indexed_at" pipeline ?


## Mappings

* Missing dynamic fields
 * administrative regions: index=no
 * bbox
 * boundary
 * names, labels, etc.

* "zip_codes"
	* no "prefix" field

* "approx_coord": missing type

* "label"
	* no "ngram" field
	* `labels.<lang>` fields
	* name, names, etc.

* "full_label"
	* search_analyzer to define ? (without prefix)
	* no "ngram" field

* "zone_type" should be "keyword"

* missing properties
	* codes: fields name/value, or use new "flattened" type
	* country_codes: keyword
	* insee: keyword
	* parent_id: no index



