# si-registry-rs

**Local fleet registry client for [SuperInstance](https://github.com/SuperInstance)**

Query repos, capabilities, and agent budgets from a Supabase-backed registry — or scan local directories for capability manifests. Built with sync Rust, zero async runtime required.

---

## Features

- **Supabase HTTP client** — list, search, and query repos/capabilities/budgets
- **Local file cache** — JSON-backed cache with configurable TTL
- **Conservation invariant** — verify `γ + η = total` budget constraints across the fleet
- **Local scanner** — discover repos by reading `CAPABILITY.toml` manifests from disk
- **Fully synchronous** — uses `ureq`, no tokio/async needed
- **Serde-compatible types** — serialize/deserialize everything as JSON

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
si-registry-rs = "0.1"
```

Or use `cargo add`:

```bash
cargo add si-registry-rs
```

## Quick Start

```rust
use si_registry_rs::client::RegistryClient;
use si_registry_rs::conservation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to your Supabase-backed registry
    let mut client = RegistryClient::new(
        "https://your-project.supabase.co",
        "your-anon-key"
    );

    // List repos
    let repos = client.list_repos(0, 50)?;
    for repo in &repos {
        println!("📦 {} ({}) — {}", repo.name, repo.language, repo.description);
    }

    // Search for a specific repo
    let results = client.search_repos("ml-inference")?;
    println!("Found {} matching repos", results.len());

    // Get a single repo
    let repo = client.get_repo("si-registry-rs")?;
    println!("Repo: {} at {}", repo.name, repo.url);

    // Check conservation invariants
    let budgets = client.list_budgets()?;
    let fleet = conservation::fleet_conservation(&budgets);
    println!("Fleet conservation holds: {}", fleet.invariant_holds);
    println!("Total γ={}, η={}, budget={}", fleet.total_gamma, fleet.total_eta, fleet.total_budget);

    Ok(())
}
```

## API Reference

### `client::RegistryClient`

The main HTTP client for interacting with the Supabase registry.

#### `RegistryClient::new(url, anon_key)`

Create a new client targeting the given Supabase project URL.

```rust
let mut client = RegistryClient::new(
    "https://abcxyz.supabase.co",
    "eyJhbGciOiJIUzI1NiIs..."
);
```

#### `client.list_repos(page, per_page)`

List repos with pagination support.

```rust
// First page, 25 per page
let repos = client.list_repos(0, 25)?;

// Second page
let more = client.list_repos(1, 25)?;
```

Each `Repo` has:

| Field         | Type     | Description              |
|---------------|----------|--------------------------|
| `name`        | `String` | Repository name          |
| `description` | `String` | Human-readable summary   |
| `language`    | `String` | Primary language         |
| `url`         | `String` | Repository URL           |
| `updated_at`  | `String` | Last update timestamp    |

#### `client.get_repo(name)`

Fetch a single repo by exact name. Returns `RegistryError::NotFound` if no match.

```rust
let repo = client.get_repo("my-agent")?;
println!("{} — {}", repo.name, repo.url);
```

#### `client.search_repos(query)`

Search repos by name or description (case-insensitive, partial match).

```rust
let results = client.search_repos("inference")?;
for r in &results {
    println!("{}: {}", r.name, r.description);
}
```

#### `client.list_capabilities()`

List all capabilities registered across the fleet.

```rust
let caps = client.list_capabilities()?;
for cap in &caps {
    println!("🔧 {} [{}] — {}", cap.name, cap.category, cap.provides);
}
```

Each `Capability` has:

| Field          | Type     | Description              |
|----------------|----------|--------------------------|
| `name`         | `String` | Capability identifier    |
| `category`     | `String` | Category grouping        |
| `provides`     | `String` | What it provides         |
| `description`  | `String` | Human-readable summary   |

#### `client.list_budgets()`

List all agent budget allocations.

```rust
let budgets = client.list_budgets()?;
for b in &budgets {
    println!("Agent {} — γ={}, η={}, total={}", b.agent_id, b.gamma, b.eta, b.total);
}
```

Each `AgentBudget` has:

| Field       | Type     | Description              |
|-------------|----------|--------------------------|
| `agent_id`  | `String` | Unique agent identifier  |
| `gamma`     | `f64`    | Gamma allocation         |
| `eta`       | `f64`    | Eta allocation           |
| `total`     | `f64`    | Total budget             |

#### `client.fleet_stats()`

Get aggregate statistics about the entire fleet.

```rust
let stats = client.fleet_stats()?;
println!("{} repos, {} capabilities, {} total budget",
    stats.total_repos, stats.total_capabilities, stats.total_budget);

for (lang, count) in &stats.languages {
    println!("  {} — {} repos", lang, count);
}
```

### Caching

Enable local file caching to reduce API calls:

```rust
use std::time::Duration;

let mut client = RegistryClient::new(
    "https://your-project.supabase.co",
    "your-anon-key"
).with_cache("/tmp/si-cache.json", Duration::from_secs(300))?;
```

All subsequent `list_repos`, `get_repo`, and `search_repos` calls will check the cache first.

### `cache::FileCache`

A standalone file-based cache with TTL support.

```rust
use si_registry_rs::cache::FileCache;
use std::time::Duration;

let mut cache = FileCache::new("/tmp/my-cache.json")?;

// Store a value for 5 minutes
cache.set("repos:page:0", "[{\"name\":\"foo\"}]", Duration::from_secs(300));

// Retrieve it
if let Some(data) = cache.get("repos:page:0") {
    println!("Cached: {}", data);
}

// Check existence
assert!(cache.contains_key("repos:page:0"));

// Remove a key
cache.remove("repos:page:0");

// Clear everything
cache.clear();

// Purge expired entries
cache.purge_expired();

// Get count of live entries
println!("{} cached items", cache.len());
```

The cache stores entries as JSON with expiry metadata:

```json
{
  "repos:page:0": {
    "value": "[{\"name\":\"foo\"}]",
    "expires_at": 1703276000
  }
}
```

### `conservation` — Budget Invariant Checks

The conservation module verifies the budget invariant: **γ + η = total** for each agent and across the entire fleet.

#### `check_conservation(budget)`

Check a single agent's budget.

```rust
use si_registry_rs::conservation;
use si_registry_rs::AgentBudget;

let budget = AgentBudget {
    agent_id: "agent-1".into(),
    gamma: 60.0,
    eta: 40.0,
    total: 100.0,
};

assert!(conservation::check_conservation(&budget));
```

#### `fleet_conservation(budgets)`

Check the fleet-wide invariant (sum of all γ + sum of all η = sum of all totals).

```rust
let budgets = client.list_budgets()?;
let fc = conservation::fleet_conservation(&budgets);

println!("Total γ: {}", fc.total_gamma);
println!("Total η: {}", fc.total_eta);
println!("Total budget: {}", fc.total_budget);
println!("Invariant holds: {}", fc.invariant_holds);
```

#### `budget_deficit(budget)`

Compute the deficit (or surplus) for a single agent.

```rust
let deficit = conservation::budget_deficit(&budget);
if deficit > 0.0 {
    println!("Agent {} is under-allocated by {}", budget.agent_id, deficit);
} else if deficit < 0.0 {
    println!("Agent {} is over-allocated by {}", budget.agent_id, -deficit);
}
```

#### `violating_agents(budgets)`

Find all agents that violate the conservation invariant.

```rust
let violators = conservation::violating_agents(&budgets);
if !violators.is_empty() {
    eprintln!("⚠️ Agents with broken budgets: {:?}", violators);
}
```

### `scan` — Local Repo Scanner

Discover local repos by scanning directories for `CAPABILITY.toml` files.

#### `scan_dir(path)`

```rust
use si_registry_rs::scan;

let repos = scan::scan_dir("/path/to/fleet")?;
for repo in &repos {
    println!("📂 {} at {}", repo.name, repo.path);
    for cap in &repo.capabilities {
        println!("  🔧 {} — {}", cap.name, cap.description);
    }
}
```

#### `CAPABILITY.toml` Format

Each subdirectory can contain a `CAPABILITY.toml`:

```toml
[[capabilities]]
name = "compute"
category = "ml"
provides = "inference"
description = "ML inference engine"

[[capabilities]]
name = "storage"
category = "data"
provides = "persistence"
description = "Persistent storage layer"
```

#### `all_capabilities(repos)`

Collect all unique capabilities from a set of local repos (deduped by name + category).

```rust
let caps = scan::all_capabilities(&repos);
println!("{} unique capabilities found", caps.len());
```

#### `find_by_capability(repos, name)`

Find repos that provide a specific capability.

```rust
let matching = scan::find_by_capability(&repos, "compute");
for repo in matching {
    println!("{} provides 'compute'", repo.name);
}
```

## Types

### `Repo`

```rust
pub struct Repo {
    pub name: String,
    pub description: String,
    pub language: String,
    pub url: String,
    pub updated_at: String,
}
```

### `Capability`

```rust
pub struct Capability {
    pub name: String,
    pub category: String,
    pub provides: String,
    pub description: String,
}
```

### `AgentBudget`

```rust
pub struct AgentBudget {
    pub agent_id: String,
    pub gamma: f64,
    pub eta: f64,
    pub total: f64,
}
```

### `FleetStats`

```rust
pub struct FleetStats {
    pub total_repos: usize,
    pub languages: HashMap<String, usize>,
    pub total_capabilities: usize,
    pub total_budget: f64,
}
```

### `LocalRepo`

```rust
pub struct LocalRepo {
    pub path: String,
    pub name: String,
    pub capabilities: Vec<Capability>,
}
```

### `FleetConservation`

```rust
pub struct FleetConservation {
    pub total_gamma: f64,
    pub total_eta: f64,
    pub total_budget: f64,
    pub invariant_holds: bool,
}
```

## Error Handling

All fallible operations return `Result<T, RegistryError>`:

```rust
use si_registry_rs::RegistryError;

match client.get_repo("nonexistent") {
    Ok(repo) => println!("Found: {}", repo.name),
    Err(RegistryError::NotFound(name)) => eprintln!("Not found: {}", name),
    Err(RegistryError::Http(msg)) => eprintln!("HTTP error: {}", msg),
    Err(RegistryError::CacheIo(msg)) => eprintln!("Cache error: {}", msg),
    Err(RegistryError::Json(e)) => eprintln!("Parse error: {}", e),
    Err(other) => eprintln!("Other: {}", other),
}
```

## Example: Full Fleet Audit

```rust
use si_registry_rs::client::RegistryClient;
use si_registry_rs::conservation;
use si_registry_rs::scan;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = RegistryClient::new(
        "https://your-project.supabase.co",
        "your-anon-key"
    ).with_cache("/tmp/si-audit-cache.json", Duration::from_secs(600))?;

    // 1. Registry overview
    let stats = client.fleet_stats()?;
    println!("=== Fleet Overview ===");
    println!("Repos: {}", stats.total_repos);
    println!("Capabilities: {}", stats.total_capabilities);
    println!("Total budget: {:.2}", stats.total_budget);
    println!();

    // 2. Language breakdown
    println!("=== Languages ===");
    for (lang, count) in &stats.languages {
        println!("  {} — {} repos", lang, count);
    }
    println!();

    // 3. Budget conservation check
    println!("=== Budget Conservation ===");
    let budgets = client.list_budgets()?;
    let fc = conservation::fleet_conservation(&budgets);
    println!("Total γ: {:.2}", fc.total_gamma);
    println!("Total η: {:.2}", fc.total_eta);
    println!("Total budget: {:.2}", fc.total_budget);
    println!("Invariant holds: {}", fc.invariant_holds);

    if !fc.invariant_holds {
        let violators = conservation::violating_agents(&budgets);
        eprintln!("⚠️ Violating agents: {:?}", violators);
        for agent_id in &violators {
            if let Some(b) = budgets.iter().find(|b| &b.agent_id == agent_id) {
                let deficit = conservation::budget_deficit(b);
                eprintln!("  {} — deficit: {:.4}", agent_id, deficit);
            }
        }
    }
    println!();

    // 4. Local scan comparison
    println!("=== Local Scan ===");
    let local = scan::scan_dir("./fleet")?;
    println!("Found {} local repos", local.len());
    let local_caps = scan::all_capabilities(&local);
    println!("{} unique local capabilities", local_caps.len());

    // 5. Cross-reference
    let remote_caps = client.list_capabilities()?;
    let remote_names: Vec<&str> = remote_caps.iter().map(|c| c.name.as_str()).collect();
    let local_only: Vec<&str> = local_caps
        .iter()
        .map(|c| c.name.as_str())
        .filter(|n| !remote_names.contains(n))
        .collect();

    if !local_only.is_empty() {
        println!("⚠️ Local-only capabilities (not in registry): {:?}", local_only);
    }

    Ok(())
}
```

## Example: Local-Only Usage (No Supabase)

```rust
use si_registry_rs::scan;
use si_registry_rs::conservation;
use si_registry_rs::cache::FileCache;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Scan local directories
    let repos = scan::scan_dir("/opt/fleet")?;
    println!("Discovered {} repos", repos.len());

    for repo in &repos {
        println!("\n📂 {} ({})", repo.name, repo.path);
        for cap in &repo.capabilities {
            println!("  🔧 {} [{}]", cap.name, cap.category);
        }
    }

    // Cache the scan results
    let mut cache = FileCache::new("/tmp/fleet-scan-cache.json")?;
    let json = serde_json::to_string(&repos)?;
    cache.set("scan:/opt/fleet", &json, Duration::from_secs(3600));

    // Build a synthetic budget to check conservation
    let budgets = vec![
        si_registry_rs::AgentBudget {
            agent_id: "local-agent-1".into(),
            gamma: 40.0,
            eta: 60.0,
            total: 100.0,
        },
    ];

    let fc = conservation::fleet_conservation(&budgets);
    assert!(fc.invariant_holds);
    println!("Local fleet conservation: {}", fc.invariant_holds);

    Ok(())
}
```

## Testing

Run the full test suite:

```bash
cargo test
```

Run with output:

```bash
cargo test -- --nocapture
```

Run only conservation tests:

```bash
cargo test conservation
```

Run only cache tests:

```bash
cargo test cache
```

## Supabase Table Schema

The client expects the following tables in your Supabase project:

### `repos`

```sql
CREATE TABLE repos (
    name TEXT PRIMARY KEY,
    description TEXT NOT NULL DEFAULT '',
    language TEXT NOT NULL DEFAULT '',
    url TEXT NOT NULL DEFAULT '',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### `capabilities`

```sql
CREATE TABLE capabilities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT '',
    provides TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT ''
);
```

### `agent_budgets`

```sql
CREATE TABLE agent_budgets (
    agent_id TEXT PRIMARY KEY,
    gamma DOUBLE PRECISION NOT NULL DEFAULT 0,
    eta DOUBLE PRECISION NOT NULL DEFAULT 0,
    total DOUBLE PRECISION NOT NULL DEFAULT 0
);
```

## Architecture

```
si-registry-rs
├── src/
│   ├── lib.rs           # Crate root, re-exports
│   ├── types.rs         # Data types and error definitions
│   ├── client.rs        # Supabase HTTP client with caching
│   ├── cache.rs         # File-based JSON cache with TTL
│   ├── conservation.rs  # Budget invariant enforcement
│   └── scan.rs          # Local directory scanner
├── Cargo.toml
└── README.md
```

### Design Decisions

- **Synchronous HTTP** — `ureq` instead of `reqwest` to avoid pulling in tokio. This is a CLI/library tool, not a web server.
- **File cache** — Simple JSON file instead of SQLite or Redis. Good enough for local/CLI use.
- **Owned types** — All types use `String` instead of `&str` references for simplicity. Clone is cheap for registry-sized data.
- **Epsilon comparison** — Conservation checks use `1e-9` tolerance for floating-point comparisons.

## License

MIT
