//! Core data types for Engram task graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The core unit of work in Engram.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    /// Unique identifier: "eg-" + 10 hex chars from content hash + entropy
    pub id: String,

    /// Short description of the work
    pub title: String,

    /// Optional longer description (markdown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Current state
    pub status: Status,

    /// Priority 0-4 (0 = critical, 4 = low)
    pub priority: u8,

    /// Freeform tags for filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// When created
    pub created_at: DateTime<Utc>,

    /// Last modification
    pub updated_at: DateTime<Utc>,

    /// When closed (if status == Closed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,

    /// Why it was closed (optional context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_reason: Option<String>,
}

/// Item status states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    InProgress,
    Blocked,
    Closed,
}

impl Status {
    /// Check if a status transition is valid.
    pub fn can_transition_to(&self, target: &Status) -> bool {
        use Status::*;
        match (self, target) {
            // From Open
            (Open, InProgress) => true,
            (Open, Blocked) => true,
            (Open, Closed) => true,

            // From InProgress
            (InProgress, Open) => true,
            (InProgress, Blocked) => true,
            (InProgress, Closed) => true,

            // From Blocked
            (Blocked, Open) => true,
            (Blocked, InProgress) => true,
            (Blocked, Closed) => true,

            // From Closed
            (Closed, Open) => true,

            // Same status = no-op, allowed
            (a, b) if a == b => true,

            _ => false,
        }
    }
}

/// Relationships between items.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    /// The item that has the dependency
    pub from_id: String,

    /// The item being depended on
    pub to_id: String,

    /// Type of relationship
    pub kind: EdgeKind,

    /// When the edge was created
    pub created_at: DateTime<Utc>,

    /// Tombstone marker for deletion
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deleted: bool,
}

/// Types of relationships between items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// to_id blocks from_id (from can't start until to closes)
    Blocks,

    /// from_id is a child of to_id (hierarchical)
    ParentChild,

    /// Informational link, no blocking semantics
    Related,
}

impl EdgeKind {
    /// Returns true if this edge type affects ready() calculation.
    pub fn is_blocking(&self) -> bool {
        matches!(self, EdgeKind::Blocks | EdgeKind::ParentChild)
    }
}

/// Validation errors for items.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    EmptyTitle,
    TitleTooLong,
    InvalidCharacters,
    InvalidPriority,
    InvalidLabel(String),
    InvalidTimestamp,
    ClosedAtWithoutClosedStatus,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::EmptyTitle => write!(f, "title cannot be empty"),
            ValidationError::TitleTooLong => write!(f, "title exceeds 500 characters"),
            ValidationError::InvalidCharacters => write!(f, "title contains control characters"),
            ValidationError::InvalidPriority => write!(f, "priority must be 0-4"),
            ValidationError::InvalidLabel(label) => {
                write!(
                    f,
                    "invalid label '{}': must be alphanumeric with hyphens/underscores",
                    label
                )
            }
            ValidationError::InvalidTimestamp => write!(f, "updated_at cannot be before created_at"),
            ValidationError::ClosedAtWithoutClosedStatus => {
                write!(f, "closed_at set but status is not Closed")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

impl Item {
    /// Validate the item's fields.
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Title: required, 1-500 chars, no control characters
        if self.title.is_empty() {
            return Err(ValidationError::EmptyTitle);
        }
        if self.title.len() > 500 {
            return Err(ValidationError::TitleTooLong);
        }
        if self.title.chars().any(|c| c.is_control()) {
            return Err(ValidationError::InvalidCharacters);
        }

        // Priority: 0-4
        if self.priority > 4 {
            return Err(ValidationError::InvalidPriority);
        }

        // Labels: alphanumeric + hyphens/underscores, no spaces
        for label in &self.labels {
            if !label.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                return Err(ValidationError::InvalidLabel(label.clone()));
            }
        }

        // Timestamps: updated_at >= created_at
        if self.updated_at < self.created_at {
            return Err(ValidationError::InvalidTimestamp);
        }

        // closed_at only if status == Closed
        if self.closed_at.is_some() && self.status != Status::Closed {
            return Err(ValidationError::ClosedAtWithoutClosedStatus);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(title: &str) -> Item {
        let now = Utc::now();
        Item {
            id: "eg-test12345".to_string(),
            title: title.to_string(),
            description: None,
            status: Status::Open,
            priority: 2,
            labels: vec![],
            created_at: now,
            updated_at: now,
            closed_at: None,
            close_reason: None,
        }
    }

    #[test]
    fn test_item_validation_valid() {
        let item = make_item("Valid title");
        assert!(item.validate().is_ok());
    }

    #[test]
    fn test_item_validation_empty_title() {
        let item = make_item("");
        assert_eq!(item.validate(), Err(ValidationError::EmptyTitle));
    }

    #[test]
    fn test_item_validation_title_too_long() {
        let item = make_item(&"x".repeat(501));
        assert_eq!(item.validate(), Err(ValidationError::TitleTooLong));
    }

    #[test]
    fn test_item_validation_control_chars() {
        let item = make_item("Title\x00with null");
        assert_eq!(item.validate(), Err(ValidationError::InvalidCharacters));
    }

    #[test]
    fn test_item_validation_invalid_priority() {
        let mut item = make_item("Valid title");
        item.priority = 5;
        assert_eq!(item.validate(), Err(ValidationError::InvalidPriority));
    }

    #[test]
    fn test_item_validation_invalid_label() {
        let mut item = make_item("Valid title");
        item.labels = vec!["valid-label".to_string(), "invalid label".to_string()];
        assert_eq!(
            item.validate(),
            Err(ValidationError::InvalidLabel("invalid label".to_string()))
        );
    }

    #[test]
    fn test_item_validation_closed_at_without_closed_status() {
        let mut item = make_item("Valid title");
        item.closed_at = Some(Utc::now());
        assert_eq!(item.validate(), Err(ValidationError::ClosedAtWithoutClosedStatus));
    }

    #[test]
    fn test_status_transitions() {
        use Status::*;

        // Valid transitions from Open
        assert!(Open.can_transition_to(&InProgress));
        assert!(Open.can_transition_to(&Blocked));
        assert!(Open.can_transition_to(&Closed));

        // Valid transitions from InProgress
        assert!(InProgress.can_transition_to(&Open));
        assert!(InProgress.can_transition_to(&Blocked));
        assert!(InProgress.can_transition_to(&Closed));

        // Valid transitions from Blocked
        assert!(Blocked.can_transition_to(&Open));
        assert!(Blocked.can_transition_to(&InProgress));
        assert!(Blocked.can_transition_to(&Closed));

        // Valid transitions from Closed (only reopen)
        assert!(Closed.can_transition_to(&Open));

        // Invalid transitions from Closed
        assert!(!Closed.can_transition_to(&InProgress));
        assert!(!Closed.can_transition_to(&Blocked));

        // Same status is always allowed
        assert!(Open.can_transition_to(&Open));
        assert!(Closed.can_transition_to(&Closed));
    }

    #[test]
    fn test_edge_kind_is_blocking() {
        assert!(EdgeKind::Blocks.is_blocking());
        assert!(EdgeKind::ParentChild.is_blocking());
        assert!(!EdgeKind::Related.is_blocking());
    }

    #[test]
    fn test_item_serialization_roundtrip() {
        let item = make_item("Test item");
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_edge_serialization_roundtrip() {
        let edge = Edge {
            from_id: "eg-child00001".to_string(),
            to_id: "eg-parent0001".to_string(),
            kind: EdgeKind::Blocks,
            created_at: Utc::now(),
            deleted: false,
        };
        let json = serde_json::to_string(&edge).unwrap();
        let deserialized: Edge = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, deserialized);
    }
}
