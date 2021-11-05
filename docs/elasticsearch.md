# Elasticsearch

We use component templates, index templates, and search templates.

This document describes the details of the templates and how they relate to search rankings. It is
the result of the [process](/docs/process/elasticsearch.md), and so it can evolve.

## Baseline

We consider the baseline to be a *basic* text base search.

### Search As You Type

The text search is primarily based on the
[Search-as-you-type](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-as-you-type.html)
Elasticsearch functionality, applied to the label.

The **name** is a text field identifying the place, while the **label** is a text field, created
before indexing into Elasticsearch (see [labels.rs](/src/labels.rs]) for details), which contains
the name and some context. For example, for the city of Amsterdam (an administrative region), we
have `{ "name": "Amsterdam", "label": "Amsterdam, Noord-Hollad, Nederland" }`.

We also have two other fields that have internationalized versions of both name and label. These are 
**names** and **labels**, and they are both key/value maps, where the key is a two letter code for
the language, and the value is the name or the label in that language. For example, for Amsterdam,
the complete set of names and labels looks like:

```json
{
  ...
  "level": 10,
  "name": "Amsterdam",
  "names": [
    { "ja": "アムステルダム" },
    { "ru": "Амстердам" }
  ],
  "label": "Amsterdam, Noord-Hollad, Nederland",
  "labels": [
    { "ja": "アムステルダム, Noord-Holland, オランダ" },
    { "it": "Amsterdam, Noord-Holland, Paesi Bassi" },
    { "fr": "Amsterdam, Hollande-Septentrionale, Pays-Bas" },
    { "ru": "Амстердам, Северная Голландия, Нидерланды" }
  ],
  "zone_type": "city",
  ...
}
```

The label field is found in the
[mimir-base](/config/elasticsearch/templates/component/mimir-base.json) component template. We add
an analyzer so that we can correctly handle elision and synonyms:

```
{
  "elasticsearch": {
    "template": {
      "settings": {
        ...
        "analysis": {
          "analyzer": {
            "label_analyzer": {
              "tokenizer": "standard",
              "filter": [ "lowercase", "asciifolding", "synonym", "elision" ],
              "char_filter": []
            }
          },
          "filter": {
            "synonym": {
              "type": "synonym",
              "synonyms": [ "cc,centre commercial", "st,saint", ... ]
            },
            "elision": {
              "type": "elision",
              "articles": [ "l", "d" ]
            }
          }
        }
      },
      "mappings": {
        "properies": {
          ...
          "label": {
            "type": "search_as_you_type",
            "analyzer": "label_analyzer"
          }
        }
      }
    }
  }
}
```

Then in the search template, we have the following:

```json
{
  "bool": {
      "boost": "{{settings.global}}",
      "should": [
          {
              "multi_match": {
                  "query": "{{query_string}}",
                  "type": "bool_prefix",
                  "fields": [
                      "label", "label._2gram", "label._3gram", "name"
                  ]
              }
          }
      ]
  }
}
```

