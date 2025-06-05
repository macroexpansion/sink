use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a text document with synchronization metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub name: String,
    pub content: String,
    pub last_modified: DateTime<Utc>,
    pub last_modified_by: Option<Uuid>,
    pub version: u64,
}

impl Document {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            content: String::new(),
            last_modified: Utc::now(),
            last_modified_by: None,
            version: 0,
        }
    }

    pub fn update_content(&mut self, content: String, client_id: Uuid, timestamp: DateTime<Utc>) {
        // Simple conflict resolution: use the latest timestamp
        if timestamp >= self.last_modified {
            self.content = content;
            self.last_modified = timestamp;
            self.last_modified_by = Some(client_id);
            self.version += 1;
        }
    }

    pub fn get_info(&self) -> crate::rpc::DocumentInfo {
        crate::rpc::DocumentInfo {
            id: self.id,
            name: self.name.clone(),
            last_modified: self.last_modified,
            content_length: self.content.len(),
        }
    }
}

/// Document store that manages all documents
#[derive(Debug, Default)]
pub struct DocumentStore {
    documents: HashMap<Uuid, Document>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn create_document(&mut self, name: String) -> &Document {
        let document = Document::new(name);
        let id = document.id;
        self.documents.insert(id, document);
        self.documents.get(&id).unwrap()
    }

    pub fn get_document(&self, id: &Uuid) -> Option<&Document> {
        self.documents.get(id)
    }

    pub fn get_document_mut(&mut self, id: &Uuid) -> Option<&mut Document> {
        self.documents.get_mut(id)
    }

    pub fn update_document(
        &mut self,
        id: &Uuid,
        content: String,
        client_id: Uuid,
        timestamp: DateTime<Utc>,
    ) -> Option<&Document> {
        if let Some(document) = self.documents.get_mut(id) {
            document.update_content(content, client_id, timestamp);
            Some(document)
        } else {
            None
        }
    }

    pub fn list_documents(&self) -> Vec<crate::rpc::DocumentInfo> {
        self.documents.values().map(|doc| doc.get_info()).collect()
    }

    pub fn document_exists(&self, id: &Uuid) -> bool {
        self.documents.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new("test.txt".to_string());
        assert_eq!(doc.name, "test.txt");
        assert_eq!(doc.content, "");
        assert_eq!(doc.version, 0);
    }

    #[test]
    fn test_document_update() {
        let mut doc = Document::new("test.txt".to_string());
        let client_id = Uuid::new_v4();
        let timestamp = Utc::now();

        doc.update_content("Hello, world!".to_string(), client_id, timestamp);

        assert_eq!(doc.content, "Hello, world!");
        assert_eq!(doc.last_modified_by, Some(client_id));
        assert_eq!(doc.version, 1);
    }

    #[test]
    fn test_document_store() {
        let mut store = DocumentStore::new();

        let doc = store.create_document("test.txt".to_string());
        let doc_id = doc.id;

        assert!(store.document_exists(&doc_id));
        assert_eq!(store.list_documents().len(), 1);

        let client_id = Uuid::new_v4();
        let updated_doc = store.update_document(
            &doc_id,
            "Updated content".to_string(),
            client_id,
            Utc::now(),
        );

        assert!(updated_doc.is_some());
        assert_eq!(updated_doc.unwrap().content, "Updated content");
    }
}
