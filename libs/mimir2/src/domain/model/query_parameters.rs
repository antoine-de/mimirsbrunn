/// Defines the parameters used to list or search documents.  We provide translation mechanism from
/// the export port (which knows about document types) to the query port which only knows about
/// indices.
#[derive(Debug, Clone)]
pub struct ListParameters {
    pub index: String,
}

impl From<super::export_parameters::ListParameters> for ListParameters {
    fn from(input: super::export_parameters::ListParameters) -> Self {
        // We get a doc_type, and we need to translate that into the name of an index.
        ListParameters {
            index: super::configuration::root_doctype(&input.doc_type),
        }
    }
}
#[derive(Debug, Clone)]
pub struct SearchParameters {
    pub indices: Vec<String>, // if you want to target all indices, use vec![munin]
    pub dsl: String,          // if you want to target all documents, use { match_all: {} }
}

impl From<super::export_parameters::SearchParameters> for SearchParameters {
    fn from(input: super::export_parameters::SearchParameters) -> Self {
        SearchParameters {
            indices: input
                .doc_types
                .iter()
                .map(|doc_type| super::configuration::root_doctype(&doc_type))
                .collect(),
            dsl: input.dsl,
        }
    }
}
