use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A repository entry in the fleet registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Repo {
    pub name: String,
    pub description: String,
    pub language: String,
    pub url: String,
    pub updated_at: String,
}

/// A capability provided by one or more repos in the fleet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capability {
    pub name: String,
    pub category: String,
    pub provides: String,
    pub description: String,
}

/// Budget allocation for a single agent within the fleet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentBudget {
    pub agent_id: String,
    pub gamma: f64,
    pub eta: f64,
    pub total: f64,
}

/// Aggregate statistics about the entire fleet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FleetStats {
    pub total_repos: usize,
    pub languages: HashMap<String, usize>,
    pub total_capabilities: usize,
    pub total_budget: f64,
}

/// A locally-discovered repository (from scan_dir).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalRepo {
    pub path: String,
    pub name: String,
    pub capabilities: Vec<Capability>,
}

/// Result of fleet-wide conservation invariant check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FleetConservation {
    pub total_gamma: f64,
    pub total_eta: f64,
    pub total_budget: f64,
    pub invariant_holds: bool,
}

/// Errors that can occur during registry operations.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Cache I/O error: {0}")]
    CacheIo(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("{0}")]
    Other(String),
}

impl From<ureq::Error> for RegistryError {
    fn from(e: ureq::Error) -> Self {
        RegistryError::Http(e.to_string())
    }
}

impl From<std::io::Error> for RegistryError {
    fn from(e: std::io::Error) -> Self {
        RegistryError::CacheIo(e.to_string())
    }
}

/// Convenience type alias for Results in this crate.
pub type Result<T> = std::result::Result<T, RegistryError>;
