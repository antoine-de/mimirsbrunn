use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street};
use serde::Serialize;

/// Generic document.
pub trait Document: Serialize {
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

// Implementations for `Document`

impl Document for Addr {
    fn id(&self) -> String {
        self.id.clone()
    }
}

impl Document for Admin {
    fn id(&self) -> String {
        self.id.clone()
    }
}

impl Document for Poi {
    fn id(&self) -> String {
        self.id.clone()
    }
}

impl Document for Stop {
    fn id(&self) -> String {
        self.id.clone()
    }
}

impl Document for Street {
    fn id(&self) -> String {
        self.id.clone()
    }
}

// Implementations for `ContainerDocument`

impl ContainerDocument for Addr {
    fn static_doc_type() -> &'static str {
        "addr"
    }
}

impl ContainerDocument for Admin {
    fn static_doc_type() -> &'static str {
        "admin"
    }
}

impl ContainerDocument for Poi {
    fn static_doc_type() -> &'static str {
        "poi"
    }
}

impl ContainerDocument for Stop {
    fn static_doc_type() -> &'static str {
        "stop"
    }
}

impl ContainerDocument for Street {
    fn static_doc_type() -> &'static str {
        "street"
    }
}
