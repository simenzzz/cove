use base64::Engine;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, GetString, ReadTxn, StateVector, TextRef, Transact, Update};

use crate::error::AppError;

/// Maximum encoded Yjs state size we'll accept or hold in memory per post.
pub const MAX_DOC_BYTES: usize = 256 * 1024;

/// Yrs text root name. Kept stable; clients bind to a `Y.Text` of the same
/// name on their side.
pub const TEXT_ROOT: &str = "content";

/// Wraps a [`yrs::Doc`] with helpers for the bytes that travel over the WS
/// protocol.
pub struct CollabDoc {
    doc: Doc,
    text: TextRef,
}

impl CollabDoc {
    fn from_doc(doc: Doc) -> Self {
        // `get_or_insert_text` may take an internal write lock — call it once
        // here, outside any transaction, so later reads can hold the read
        // transaction without deadlocking.
        let text = doc.get_or_insert_text(TEXT_ROOT);
        Self { doc, text }
    }

    /// Build an empty document.
    pub fn new() -> Self {
        Self::from_doc(Doc::new())
    }

    /// Hydrate from a previously persisted snapshot. Empty input yields an
    /// empty doc (used when bootstrapping a fresh draft).
    pub fn from_snapshot(state_b64: &str) -> Result<Self, AppError> {
        let doc = Doc::new();
        if state_b64.is_empty() {
            return Ok(Self::from_doc(doc));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(state_b64)
            .map_err(|e| AppError::BadRequest(format!("Invalid snapshot base64: {e}")))?;
        if bytes.len() > MAX_DOC_BYTES {
            return Err(AppError::BadRequest("Snapshot exceeds size limit".into()));
        }
        let update = Update::decode_v1(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Invalid Yjs update: {e}")))?;
        {
            let mut txn = doc.transact_mut();
            txn.apply_update(update)
                .map_err(|e| AppError::BadRequest(format!("Failed to apply update: {e}")))?;
        }
        Ok(Self::from_doc(doc))
    }

    /// Apply a remote update (base64 of Yjs v1 update bytes).
    pub fn apply_update(&self, update_b64: &str) -> Result<(), AppError> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(update_b64)
            .map_err(|e| AppError::BadRequest(format!("Invalid update base64: {e}")))?;
        if bytes.len() > MAX_DOC_BYTES {
            return Err(AppError::BadRequest("Update exceeds size limit".into()));
        }
        let update = Update::decode_v1(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Invalid Yjs update: {e}")))?;
        let mut txn = self.doc.transact_mut();
        txn.apply_update(update)
            .map_err(|e| AppError::BadRequest(format!("Failed to apply update: {e}")))
    }

    /// Encode the full document state as a Yjs v1 update (base64).
    pub fn encode_state(&self) -> String {
        let txn = self.doc.transact();
        let bytes = txn.encode_state_as_update_v1(&StateVector::default());
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    /// Encode the current state vector (base64). Clients use this to request
    /// a diff via `Y.encodeStateAsUpdate(doc, stateVector)`.
    pub fn encode_state_vector(&self) -> String {
        let txn = self.doc.transact();
        let bytes = txn.state_vector().encode_v1();
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    /// Extract the plain text from the canonical text root. Used at publish
    /// time to freeze immutable `published_content`.
    pub fn text(&self) -> String {
        let txn = self.doc.transact();
        self.text.get_string(&txn)
    }

    /// Test/internal helper: get a mutable handle to the underlying Doc + Text.
    #[cfg(test)]
    fn text_ref(&self) -> &TextRef {
        &self.text
    }

    #[cfg(test)]
    fn doc_ref(&self) -> &Doc {
        &self.doc
    }
}

impl Default for CollabDoc {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use yrs::Text;

    /// Make a doc, insert some text, return its encoded full state.
    fn encode_with_text(text: &str) -> String {
        let doc = CollabDoc::new();
        let inner = doc.text_ref();
        let mut txn = doc.doc_ref().transact_mut();
        inner.insert(&mut txn, 0, text);
        drop(txn);
        doc.encode_state()
    }

    #[test]
    fn empty_doc_round_trips() {
        let doc = CollabDoc::new();
        assert_eq!(doc.text(), "");
        let snap = doc.encode_state();
        let restored = CollabDoc::from_snapshot(&snap).unwrap();
        assert_eq!(restored.text(), "");
    }

    #[test]
    fn snapshot_round_trip_preserves_text() {
        let snap = encode_with_text("hello world");
        let restored = CollabDoc::from_snapshot(&snap).unwrap();
        assert_eq!(restored.text(), "hello world");
    }

    #[test]
    fn merging_two_concurrent_updates_converges() {
        // Doc A: starts with "abc"
        let doc_a = CollabDoc::new();
        {
            let mut txn = doc_a.doc_ref().transact_mut();
            doc_a.text_ref().insert(&mut txn, 0, "abc");
        }
        let baseline = doc_a.encode_state();

        // Doc B starts from the same baseline as A
        let doc_b = CollabDoc::from_snapshot(&baseline).unwrap();

        // A appends "X" at end; B appends "Y" at end concurrently
        {
            let mut txn = doc_a.doc_ref().transact_mut();
            doc_a.text_ref().insert(&mut txn, 3, "X");
        }
        {
            let mut txn = doc_b.doc_ref().transact_mut();
            doc_b.text_ref().insert(&mut txn, 3, "Y");
        }

        // Exchange updates
        let a_state = doc_a.encode_state();
        let b_state = doc_b.encode_state();
        doc_a.apply_update(&b_state).unwrap();
        doc_b.apply_update(&a_state).unwrap();

        // Both must converge to the same final text
        assert_eq!(doc_a.text(), doc_b.text());
        // The merged text must contain both inserts
        let merged = doc_a.text();
        assert!(merged.contains('X'), "merged text {merged:?} missing X");
        assert!(merged.contains('Y'), "merged text {merged:?} missing Y");
    }

    #[test]
    fn apply_update_rejects_oversize_payload() {
        let doc = CollabDoc::new();
        // 257 KB of arbitrary bytes, base64-encoded.
        let big = base64::engine::general_purpose::STANDARD.encode(vec![0u8; MAX_DOC_BYTES + 1]);
        let err = doc.apply_update(&big).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn apply_update_rejects_malformed_base64() {
        let doc = CollabDoc::new();
        let err = doc.apply_update("!!!not-base64!!!").unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn snapshot_rejects_oversize_payload() {
        let big = base64::engine::general_purpose::STANDARD.encode(vec![0u8; MAX_DOC_BYTES + 1]);
        match CollabDoc::from_snapshot(&big) {
            Err(AppError::BadRequest(_)) => {}
            Err(other) => panic!("expected BadRequest, got {other:?}"),
            Ok(_) => panic!("expected BadRequest, got Ok"),
        }
    }
}
