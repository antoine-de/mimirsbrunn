use serde::{de::DeserializeOwned, Serialize};

/// Generic document.
pub trait Document: DeserializeOwned + Serialize {
    // TODO: Do we need to use an owned string here?
    /// Unique identifier for the document.
    fn id(&self) -> String;
}

/// A type of document with a fixed type.
///
/// A collection of this kind of document has a consistent schema and can hence
/// be used to generate a container.
pub trait ContainerDocument: Document {
    fn static_doc_type() -> &'static str;
}
