/// This port defines a method to list documents in storage

// TODO: this is probably redundant with what is in Query
#[derive(Debug, Clone)]
pub struct ListParameters {
    pub doc_type: String,
}
