//! Session Management
//!
//! Manages agent sessions with conversation history and state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::message::Conversation;

/// Unique session identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Session title (auto-generated or user-set)
    pub title: Option<String>,
    
    /// User/owner ID
    pub user_id: Option<String>,
    
    /// Model used for this session
    pub model: String,
    
    /// Custom tags
    pub tags: Vec<String>,
    
    /// Extra key-value metadata
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            title: None,
            user_id: None,
            model: "llama3.2".into(),
            tags: Vec::new(),
            extra: std::collections::HashMap::new(),
        }
    }
}

/// A complete agent session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier
    pub id: SessionId,
    
    /// Conversation history
    pub conversation: Conversation,
    
    /// Session metadata
    pub metadata: SessionMetadata,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Last activity timestamp
    pub updated_at: DateTime<Utc>,
    
    /// Whether session is active
    pub active: bool,
}

impl Session {
    /// Create a new session
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            conversation: Conversation::new(),
            metadata: SessionMetadata::default(),
            created_at: now,
            updated_at: now,
            active: true,
        }
    }
    
    /// Create with specific ID
    pub fn with_id(id: SessionId) -> Self {
        let mut session = Self::new();
        session.id = id;
        session
    }
    
    /// Create with system prompt
    pub fn with_system_prompt(system_prompt: impl Into<String>) -> Self {
        let mut session = Self::new();
        session.conversation = Conversation::with_system_prompt(system_prompt);
        session
    }
    
    /// Update the activity timestamp
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
    
    /// Set session title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.metadata.title = Some(title.into());
        self.touch();
    }
    
    /// Get or generate title
    pub fn title(&self) -> String {
        self.metadata.title.clone().unwrap_or_else(|| {
            // Generate from first user message
            self.conversation
                .messages()
                .iter()
                .find(|m| m.role == crate::message::Role::User)
                .map(|m| {
                    let preview: String = m.content.chars().take(50).collect();
                    if m.content.len() > 50 {
                        format!("{}...", preview)
                    } else {
                        preview
                    }
                })
                .unwrap_or_else(|| format!("Session {}", &self.id.0[..8]))
        })
    }
    
    /// End the session
    pub fn end(&mut self) {
        self.active = false;
        self.touch();
    }
    
    /// Message count
    pub fn message_count(&self) -> usize {
        self.conversation.len()
    }
    
    /// Duration since creation
    pub fn duration(&self) -> chrono::Duration {
        self.updated_at - self.created_at
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Session store trait for persistence
pub trait SessionStore: Send + Sync {
    /// Save a session
    fn save(&self, session: &Session) -> crate::Result<()>;
    
    /// Load a session by ID
    fn load(&self, id: &SessionId) -> crate::Result<Option<Session>>;
    
    /// Delete a session
    fn delete(&self, id: &SessionId) -> crate::Result<()>;
    
    /// List sessions for a user
    fn list(&self, user_id: Option<&str>, limit: usize) -> crate::Result<Vec<Session>>;
}

/// In-memory session store (for development/testing)
pub struct MemorySessionStore {
    sessions: std::sync::RwLock<std::collections::HashMap<SessionId, Session>>,
}

impl Default for MemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl SessionStore for MemorySessionStore {
    fn save(&self, session: &Session) -> crate::Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session.id.clone(), session.clone());
        Ok(())
    }
    
    fn load(&self, id: &SessionId) -> crate::Result<Option<Session>> {
        let sessions = self.sessions.read().unwrap();
        Ok(sessions.get(id).cloned())
    }
    
    fn delete(&self, id: &SessionId) -> crate::Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(id);
        Ok(())
    }
    
    fn list(&self, user_id: Option<&str>, limit: usize) -> crate::Result<Vec<Session>> {
        let sessions = self.sessions.read().unwrap();
        let mut result: Vec<_> = sessions
            .values()
            .filter(|s| {
                user_id.map_or(true, |uid| s.metadata.user_id.as_deref() == Some(uid))
            })
            .cloned()
            .collect();
        
        // Sort by updated_at descending
        result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        result.truncate(limit);
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new();
        assert!(session.active);
        assert_eq!(session.message_count(), 0);
    }

    #[test]
    fn test_memory_store() {
        let store = MemorySessionStore::new();
        let session = Session::new();
        let id = session.id.clone();
        
        store.save(&session).unwrap();
        
        let loaded = store.load(&id).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, id);
    }
}
