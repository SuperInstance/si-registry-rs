use crate::types::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A single cached entry with expiry metadata.
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    value: String,
    expires_at: u64,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }
}

/// A simple JSON-backed file cache with TTL support.
///
/// ```no_run
/// use si_registry_rs::cache::FileCache;
/// use std::time::Duration;
///
/// let mut cache = FileCache::new("/tmp/si-registry-cache.json").unwrap();
/// cache.set("key", "value", Duration::from_secs(300));
/// assert_eq!(cache.get("key"), Some("value".to_string()));
/// ```
#[derive(Debug)]
pub struct FileCache {
    path: PathBuf,
    entries: std::collections::HashMap<String, CacheEntry>,
}

impl FileCache {
    /// Create or open a file cache at the given path.
    pub fn new(path: &str) -> Result<Self> {
        let path = PathBuf::from(path);
        let entries = if path.exists() {
            let data = fs::read_to_string(&path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            std::collections::HashMap::new()
        };
        Ok(Self { path, entries })
    }

    /// Retrieve a cached value by key. Returns `None` if missing or expired.
    pub fn get(&mut self, key: &str) -> Option<String> {
        self.load_if_needed();
        if let Some(entry) = self.entries.get(key) {
            if entry.is_expired() {
                self.entries.remove(key);
                self.persist().ok();
                None
            } else {
                Some(entry.value.clone())
            }
        } else {
            None
        }
    }

    /// Store a value with a TTL.
    pub fn set(&mut self, key: &str, value: &str, ttl: Duration) {
        self.load_if_needed();
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + ttl.as_secs();
        self.entries.insert(
            key.to_string(),
            CacheEntry {
                value: value.to_string(),
                expires_at,
            },
        );
        self.persist().ok();
    }

    /// Remove a key from the cache.
    pub fn remove(&mut self, key: &str) {
        self.load_if_needed();
        self.entries.remove(key);
        self.persist().ok();
    }

    /// Clear all entries from the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.persist().ok();
    }

    /// Return the number of non-expired entries.
    pub fn len(&mut self) -> usize {
        self.load_if_needed();
        self.entries.retain(|_, v| !v.is_expired());
        self.entries.len()
    }

    /// Check if the cache is empty (after purging expired entries).
    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    /// Check if a key exists and is not expired.
    pub fn contains_key(&mut self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Purge all expired entries from the cache.
    pub fn purge_expired(&mut self) {
        self.load_if_needed();
        self.entries.retain(|_, v| !v.is_expired());
        self.persist().ok();
    }

    fn load_if_needed(&mut self) {
        // Only reload from disk if we haven't loaded yet (entries empty and file exists)
        // For simplicity, we work in memory after initial load
    }

    /// Get the raw number of entries (including expired, no purge).
    pub fn raw_len(&self) -> usize {
        self.entries.len()
    }

    fn persist(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.path, data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cache_set_get() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let mut cache = FileCache::new(path.to_str().unwrap()).unwrap();
        cache.set("key1", "value1", Duration::from_secs(60));
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
    }

    #[test]
    fn test_cache_miss() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let mut cache = FileCache::new(path.to_str().unwrap()).unwrap();
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_cache_ttl_expired() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let mut cache = FileCache::new(path.to_str().unwrap()).unwrap();
        cache.set("short", "data", Duration::from_millis(10));
        thread::sleep(Duration::from_millis(50));
        assert_eq!(cache.get("short"), None);
    }

    #[test]
    fn test_cache_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let mut cache = FileCache::new(path.to_str().unwrap()).unwrap();
        cache.set("key", "v1", Duration::from_secs(60));
        cache.set("key", "v2", Duration::from_secs(60));
        assert_eq!(cache.get("key"), Some("v2".to_string()));
    }

    #[test]
    fn test_cache_remove() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let mut cache = FileCache::new(path.to_str().unwrap()).unwrap();
        cache.set("key", "val", Duration::from_secs(60));
        cache.remove("key");
        assert_eq!(cache.get("key"), None);
    }

    #[test]
    fn test_cache_clear() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let mut cache = FileCache::new(path.to_str().unwrap()).unwrap();
        cache.set("a", "1", Duration::from_secs(60));
        cache.set("b", "2", Duration::from_secs(60));
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_len() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let mut cache = FileCache::new(path.to_str().unwrap()).unwrap();
        assert!(cache.is_empty());
        cache.set("a", "1", Duration::from_secs(60));
        cache.set("b", "2", Duration::from_secs(60));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_contains_key() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let mut cache = FileCache::new(path.to_str().unwrap()).unwrap();
        cache.set("key", "val", Duration::from_secs(60));
        assert!(cache.contains_key("key"));
        assert!(!cache.contains_key("other"));
    }

    #[test]
    fn test_cache_persist_and_reload() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");

        // Write
        {
            let mut cache = FileCache::new(path.to_str().unwrap()).unwrap();
            cache.set("persisted", "yes", Duration::from_secs(300));
        }

        // Reload
        let mut cache2 = FileCache::new(path.to_str().unwrap()).unwrap();
        assert_eq!(cache2.get("persisted"), Some("yes".to_string()));
    }

    #[test]
    fn test_cache_purge_expired() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let mut cache = FileCache::new(path.to_str().unwrap()).unwrap();
        cache.set("expire_me", "gone", Duration::from_millis(10));
        cache.set("keep_me", "here", Duration::from_secs(300));
        thread::sleep(Duration::from_millis(50));
        cache.purge_expired();
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get("keep_me"), Some("here".to_string()));
    }

    #[test]
    fn test_cache_creates_parent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("a/b/c/cache.json");
        let mut cache = FileCache::new(nested.to_str().unwrap()).unwrap();
        cache.set("x", "y", Duration::from_secs(10));
        assert_eq!(cache.get("x"), Some("y".to_string()));
    }
}
