use crate::cache::FileCache;
use crate::types::{
    AgentBudget, Capability, FleetStats, RegistryError, Repo, Result,
};
use std::collections::HashMap;
use std::time::Duration;

/// HTTP client for querying a Supabase-backed fleet registry.
pub struct RegistryClient {
    base_url: String,
    anon_key: String,
    cache: Option<FileCache>,
    cache_ttl: Duration,
}

impl RegistryClient {
    /// Create a new registry client pointing at the given Supabase URL.
    pub fn new(url: &str, anon_key: &str) -> Self {
        Self {
            base_url: url.trim_end_matches('/').to_string(),
            anon_key: anon_key.to_string(),
            cache: None,
            cache_ttl: Duration::from_secs(300),
        }
    }

    /// Enable local file caching with the given path and TTL.
    pub fn with_cache(mut self, path: &str, ttl: Duration) -> Result<Self> {
        self.cache = Some(FileCache::new(path)?);
        self.cache_ttl = ttl;
        Ok(self)
    }

    /// List repos with pagination.
    pub fn list_repos(&mut self, page: usize, per_page: usize) -> Result<Vec<Repo>> {
        let cache_key = format!("repos:{}:{}", page, per_page);
        if let Some(ref mut cache) = self.cache {
            if let Some(data) = cache.get(&cache_key) {
                return Ok(serde_json::from_str(&data)?);
            }
        }

        let offset = page * per_page;
        let url = format!(
            "{}/rest/v1/repos?select=*&order=name.asc&limit={}&offset={}",
            self.base_url, per_page, offset
        );

        let repos: Vec<Repo> = self.get(&url)?;

        if let Some(ref mut cache) = self.cache {
            if let Ok(data) = serde_json::to_string(&repos) {
                cache.set(&cache_key, &data, self.cache_ttl);
            }
        }

        Ok(repos)
    }

    /// Get a single repo by name.
    pub fn get_repo(&mut self, name: &str) -> Result<Repo> {
        let cache_key = format!("repo:{}", name);
        if let Some(ref mut cache) = self.cache {
            if let Some(data) = cache.get(&cache_key) {
                return Ok(serde_json::from_str(&data)?);
            }
        }

        let url = format!(
            "{}/rest/v1/repos?name=eq.{}&select=*&limit=1",
            self.base_url, name
        );

        let mut repos: Vec<Repo> = self.get(&url)?;
        let repo = repos.pop().ok_or_else(|| {
            RegistryError::NotFound(format!("Repo '{}' not found", name))
        })?;

        if let Some(ref mut cache) = self.cache {
            if let Ok(data) = serde_json::to_string(&repo) {
                cache.set(&cache_key, &data, self.cache_ttl);
            }
        }

        Ok(repo)
    }

    /// Search repos by a text query (matches name or description).
    pub fn search_repos(&mut self, query: &str) -> Result<Vec<Repo>> {
        let cache_key = format!("search:{}", query);
        if let Some(ref mut cache) = self.cache {
            if let Some(data) = cache.get(&cache_key) {
                return Ok(serde_json::from_str(&data)?);
            }
        }

        let url = format!(
            "{}/rest/v1/repos?or=(name.ilike.%25{}%25,description.ilike.%25{}%25)&select=*&order=name.asc",
            self.base_url, query, query
        );

        let repos: Vec<Repo> = self.get(&url)?;

        if let Some(ref mut cache) = self.cache {
            if let Ok(data) = serde_json::to_string(&repos) {
                cache.set(&cache_key, &data, self.cache_ttl);
            }
        }

        Ok(repos)
    }

    /// List all capabilities in the registry.
    pub fn list_capabilities(&mut self) -> Result<Vec<Capability>> {
        let url = format!(
            "{}/rest/v1/capabilities?select=*&order=name.asc",
            self.base_url
        );
        self.get(&url)
    }

    /// List all agent budgets.
    pub fn list_budgets(&mut self) -> Result<Vec<AgentBudget>> {
        let url = format!(
            "{}/rest/v1/agent_budgets?select=*&order=agent_id.asc",
            self.base_url
        );
        self.get(&url)
    }

    /// Get aggregate fleet statistics.
    pub fn fleet_stats(&mut self) -> Result<FleetStats> {
        let repos = self.list_repos(0, 1000)?;
        let caps = self.list_capabilities()?;
        let budgets = self.list_budgets()?;

        let mut languages: HashMap<String, usize> = HashMap::new();
        for repo in &repos {
            *languages.entry(repo.language.clone()).or_insert(0) += 1;
        }

        let total_budget: f64 = budgets.iter().map(|b| b.total).sum();

        Ok(FleetStats {
            total_repos: repos.len(),
            languages,
            total_capabilities: caps.len(),
            total_budget,
        })
    }

    fn get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let response = ureq::Agent::new_with_defaults()
            .get(url)
            .header("apikey", &self.anon_key)
            .header("Authorization", &format!("Bearer {}", self.anon_key))
            .call()
            .map_err(|e| RegistryError::Http(e.to_string()))?;

        let body = response.into_body().read_to_string().map_err(|e| {
            RegistryError::Http(format!("Failed to read response body: {}", e))
        })?;

        serde_json::from_str(&body).map_err(Into::into)
    }
}
