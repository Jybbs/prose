//! In-memory store of each open buffer's live text and version, keyed by URI.

use std::collections::HashMap;

use lsp_types::Uri;

/// Tracks the editor's current text and version for every open document.
/// Full document sync means an open or change carries the whole buffer, so
/// both collapse to a single insert and a close drops the entry.
#[derive(Default)]
pub(super) struct DocumentStore {
    docs: HashMap<Uri, Document>,
}

/// One open buffer's live text paired with the version the editor stamped.
pub(super) struct Document {
    pub(super) text: String,
    pub(super) version: i32,
}

impl DocumentStore {
    pub(super) fn get(&self, uri: &Uri) -> Option<&Document> {
        self.docs.get(uri)
    }

    pub(super) fn remove(&mut self, uri: &Uri) {
        self.docs.remove(uri);
    }

    pub(super) fn set(&mut self, uri: Uri, text: String, version: i32) {
        self.docs.insert(uri, Document { text, version });
    }

    /// Returns every tracked URI, for republishing after a config change.
    pub(super) fn uris(&self) -> Vec<Uri> {
        self.docs.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::uri;

    #[test]
    fn get_returns_none_for_absent_uri() {
        let store = DocumentStore::default();
        assert!(store.get(&uri("file:///a.py")).is_none());
    }

    #[test]
    fn remove_drops_the_entry() {
        let mut store = DocumentStore::default();
        store.set(uri("file:///a.py"), "x = 1\n".to_owned(), 1);
        store.remove(&uri("file:///a.py"));
        assert!(store.get(&uri("file:///a.py")).is_none());
    }

    #[test]
    fn set_overwrites_existing_text_and_version() {
        let mut store = DocumentStore::default();
        store.set(uri("file:///a.py"), "x = 1\n".to_owned(), 1);
        store.set(uri("file:///a.py"), "y = 2\n".to_owned(), 2);
        let doc = store.get(&uri("file:///a.py")).expect("present");
        assert_eq!(doc.text, "y = 2\n");
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut store = DocumentStore::default();
        store.set(uri("file:///a.py"), "x = 1\n".to_owned(), 7);
        let doc = store.get(&uri("file:///a.py")).expect("present");
        assert_eq!(doc.text, "x = 1\n");
        assert_eq!(doc.version, 7);
    }

    #[test]
    fn uris_lists_every_open_document() {
        let mut store = DocumentStore::default();
        store.set(uri("file:///a.py"), "a\n".to_owned(), 1);
        store.set(uri("file:///b.py"), "b\n".to_owned(), 1);
        let mut uris: Vec<String> = store.uris().iter().map(|u| u.as_str().to_owned()).collect();
        uris.sort();
        assert_eq!(uris, ["file:///a.py", "file:///b.py"]);
    }
}
