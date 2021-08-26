pub trait Document: serde::Serialize {
    fn doc_type(&self) -> &'static str;

    // TODO Maybe returning a String is too restrictive, we
    // could have a DocumentKey?
    /// provides the id of the document, must be unique in the document container.
    fn id(&self) -> String;
}

#[cfg(test)]
pub mod tests {
    use super::Document;
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct Book {
        name: String,
        isbn: String,
    }

    impl Document for Book {
        fn doc_type(&self) -> &'static str {
            "book"
        }

        fn id(&self) -> String {
            self.isbn.clone()
        }
    }
}
