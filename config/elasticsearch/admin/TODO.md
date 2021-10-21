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


