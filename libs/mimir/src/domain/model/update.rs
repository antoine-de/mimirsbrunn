#[derive(Clone, Debug)]
pub enum UpdateOperation {
    /// Update a field `ident` with given value
    Set {
        ident: String,
        value: serde_json::Value,
    },
}
