use serde::Serialize;
use std::rc::Rc;
use std::sync::Arc;

pub trait Document: Serialize {
    const IS_GEO_DATA: bool;

    const DOC_TYPE: &'static str;

    // TODO Maybe returning a String is too restrictive, we
    // could have a DocumentKey?
    /// provides the id of the document, must be unique in the document container.
    fn id(&self) -> String;
}

impl<'a, T: Document> Document for &'a T {
    const IS_GEO_DATA: bool = T::IS_GEO_DATA;

    const DOC_TYPE: &'static str = T::DOC_TYPE;

    fn id(&self) -> String {
        T::id(self)
    }
}

impl<T: Document> Document for Rc<T> {
    const IS_GEO_DATA: bool = T::IS_GEO_DATA;

    const DOC_TYPE: &'static str = T::DOC_TYPE;

    fn id(&self) -> String {
        T::id(self)
    }
}

impl<T: Document> Document for Arc<T> {
    const IS_GEO_DATA: bool = T::IS_GEO_DATA;

    const DOC_TYPE: &'static str = T::DOC_TYPE;

    fn id(&self) -> String {
        T::id(self)
    }
}

pub fn doc_type<T: Document>() -> &'static str {
    T::DOC_TYPE
}

#[cfg(test)]
pub mod tests {

    // Here we define an example of a document that will be used for testing in other parts of the
    // project.
    use serde::Serialize;

    use super::Document;

    #[derive(Debug, Clone, Serialize, PartialEq)]
    pub struct Book {
        pub isbn: String,
        pub title: String,
    }

    impl Document for Book {
        const DOC_TYPE: &'static str = "book";

        const IS_GEO_DATA: bool = false;

        fn id(&self) -> String {
            self.isbn.clone()
        }
    }
}
