//! In-memory store of each open buffer's live text, keyed by URI.

use std::collections::HashMap;

use lsp_types::Uri;

/// Tracks the editor's current text for every open document. Full
/// document sync means an open or change carries the whole buffer, so
/// both collapse to a single insert and a close drops the entry.
#[derive(Default)]
pub(super) struct DocumentStore {
    docs: HashMap<Uri, String>,
}

impl DocumentStore {
    pub(super) fn get(&self, uri: &Uri) -> Option<&str> {
        self.docs.get(uri).map(String::as_str)
    }

    pub(super) fn remove(&mut self, uri: &Uri) {
        self.docs.remove(uri);
    }

    pub(super) fn set(&mut self, uri: Uri, text: String) {
        self.docs.insert(uri, text);
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn uri(s: &str) -> Uri {
        Uri::from_str(s).expect("valid uri")
    }

    #[test]
    fn get_returns_none_for_absent_uri() {
        let store = DocumentStore::default();
        assert_eq!(store.get(&uri("file:///a.py")), None);
    }

    #[test]
    fn remove_drops_the_entry() {
        let mut store = DocumentStore::default();
        store.set(uri("file:///a.py"), "x = 1\n".to_owned());
        store.remove(&uri("file:///a.py"));
        assert_eq!(store.get(&uri("file:///a.py")), None);
    }

    #[test]
    fn set_overwrites_existing_text() {
        let mut store = DocumentStore::default();
        store.set(uri("file:///a.py"), "x = 1\n".to_owned());
        store.set(uri("file:///a.py"), "y = 2\n".to_owned());
        assert_eq!(store.get(&uri("file:///a.py")), Some("y = 2\n"));
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut store = DocumentStore::default();
        store.set(uri("file:///a.py"), "x = 1\n".to_owned());
        assert_eq!(store.get(&uri("file:///a.py")), Some("x = 1\n"));
    }
}
