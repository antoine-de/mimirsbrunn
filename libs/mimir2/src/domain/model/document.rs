// use erased_serde::Serialize as ErasedSerialize;
// use serde::Serialize;
// use std::rc::Rc;
// use std::sync::Arc;

pub trait Document: erased_serde::Serialize {
    fn doc_type(&self) -> &'static str;

    // TODO Maybe returning a String is too restrictive, we
    // could have a DocumentKey?
    /// provides the id of the document, must be unique in the document container.
    fn id(&self) -> String;
}

// impl<'a, T: Document> Document for &'a T {
//     fn doc_type(&self) -> &'static str {
//         T::doc_type(self)
//     }
//
//     fn id(&self) -> String {
//         T::id(self)
//     }
// }
//
// impl<T: Document> Document for Rc<T> {
//     fn doc_type(&self) -> &'static str {
//         T::doc_type(self)
//     }
//
//     fn id(&self) -> String {
//         T::id(self)
//     }
// }
//
// impl<T: Document> Document for Arc<T> {
//     fn doc_type(&self) -> &'static str {
//         T::doc_type(self)
//     }
//
//     fn id(&self) -> String {
//         T::id(self)
//     }
// }

// impl<T: Document> Document for Box<T> {
//     fn doc_type(&self) -> &'static str {
//         T::doc_type(self)
//     }
//
//     fn id(&self) -> String {
//         T::id(self)
//     }
// }

erased_serde::serialize_trait_object!(Document);
