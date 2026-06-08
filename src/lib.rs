//! # si-registry-rs
//!
//! Local fleet registry client for SuperInstance.
//!
//! Query repos, capabilities, and agent budgets from a Supabase-backed registry,
//! or scan local directories for capability manifests.
//!
//! ## Quick Start
//!
//! ```no_run
//! use si_registry_rs::client::RegistryClient;
//! use si_registry_rs::conservation;
//!
//! let mut client = RegistryClient::new("https://your-project.supabase.co", "your-anon-key");
//! let repos = client.list_repos(0, 50).unwrap();
//! let budgets = client.list_budgets().unwrap();
//! let fc = conservation::fleet_conservation(&budgets);
//! println!("Fleet conservation holds: {}", fc.invariant_holds);
//! ```

pub mod cache;
pub mod client;
pub mod conservation;
pub mod scan;
pub mod types;

pub use types::{
    AgentBudget, Capability, FleetConservation, FleetStats, LocalRepo, RegistryError, Repo,
};
