//! Pluggable discovery providers.
//!
//! Every infrastructure connector (Terraform, Kubernetes, Airflow, OTLP, and
//! anything contributed later) implements one small trait, [`DiscoveryProvider`],
//! and registers itself with [`default_registry`]. That is the *entire* contract
//! a contributor has to satisfy to teach PyBlastRadius about a new source:
//!
//! 1. Add `src/discovery/<something>.rs` with a struct that implements
//!    [`DiscoveryProvider`] (`scheme`, `description`, `discover`).
//! 2. Write a `pub fn <something>_provider(params: &ProviderParams) -> Result<Box<dyn DiscoveryProvider>>`
//!    factory in the same file that builds your struct from [`ProviderParams`].
//! 3. Add one `registry.register(...)` line to [`default_registry`].
//!
//! No changes to the CLI, config loader, or the REST API are required: they all
//! resolve a scheme (`terraform`, `kubernetes`, `airflow`, `otlp`, ...) through
//! the registry, so a newly registered scheme works in `scan <scheme>://...`,
//! in `blastradius.yml`, and in `POST /api/v1/scan` automatically.
//!
//! The provider contract is deliberately uniform so no connector is special:
//! a provider is given a bag of string [`ProviderParams`] and returns a
//! [`Graph`]. Whether it parses local files, binds a network receiver, or calls
//! a remote API is an implementation detail the rest of the system never sees.

pub mod airflow;
pub mod kubernetes;
#[cfg(feature = "otlp-traces")]
pub mod otlp;
pub mod terraform;

use crate::error::{Error, Result};
use crate::graph::Graph;
use std::collections::BTreeMap;
use std::collections::HashMap;

/// A pluggable infrastructure discovery connector.
///
/// Implement this trait to teach PyBlastRadius to read a new dependency source.
/// The three methods are intentionally tiny so that adding a provider is a
/// mechanical, low-risk change for outside contributors.
pub trait DiscoveryProvider {
    /// Stable scheme identifier used in URIs (`terraform://path=...`), in
    /// `blastradius.yml` (`type: terraform`), and by the REST API. This is the
    /// key the provider is registered under, so it must be unique and stable.
    fn scheme(&self) -> &'static str;

    /// One-line, honest description of what this provider reads and what it
    /// produces (including its known limitations). Surfaced by
    /// `available_providers()` and the `providers` CLI listing.
    fn description(&self) -> &'static str;

    /// Build a dependency graph from this provider's configured source.
    ///
    /// Providers must be honest: if there is nothing real to report (an empty
    /// directory, a collection window with no spans), return an empty graph
    /// rather than fabricated demo data.
    fn discover(&self) -> Result<Graph>;
}

/// The string-keyed parameters a provider is constructed from.
///
/// Discovery sources are configured as flat strings in almost every entry point
/// (URI query pairs, `blastradius.yml` keys, JSON request bodies), so providers
/// receive a uniform `ProviderParams` instead of each hand-rolling its own
/// parsing. This is what lets one factory signature fit every connector.
#[derive(Debug, Clone, Default)]
pub struct ProviderParams {
    fields: HashMap<String, String>,
}

impl ProviderParams {
    /// An empty parameter set.
    pub fn new() -> Self {
        ProviderParams {
            fields: HashMap::new(),
        }
    }

    /// Build from anything iterable of `(key, value)` string pairs.
    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        ProviderParams {
            fields: pairs
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }

    /// Insert a single parameter (builder style).
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Raw lookup, returning the value only if present and non-empty.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .get(key)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    /// Value for `key`, or `default` when absent/empty.
    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).unwrap_or(default)
    }

    /// Parse `key` as an integer, returning `default` when absent/empty and an
    /// error when present but malformed (so typos fail loudly instead of being
    /// silently ignored).
    pub fn get_parsed<T>(&self, key: &str, default: T) -> Result<T>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        match self.get(key) {
            None => Ok(default),
            Some(raw) => raw.parse::<T>().map_err(|e| {
                Error::invalid_argument(format!("invalid value for '{key}': {raw:?} ({e})"))
            }),
        }
    }

    /// Split a delimited list parameter (e.g. `namespaces=ns1;ns2`). Accepts
    /// either `;` or `,` as the separator, ignores empty entries, and returns
    /// `None` when the key is absent so callers can tell "unset" from "empty".
    pub fn get_list(&self, key: &str) -> Option<Vec<String>> {
        let raw = self.get(key)?;
        let items: Vec<String> = raw
            .split([';', ','])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if items.is_empty() {
            None
        } else {
            Some(items)
        }
    }

    /// First present, non-empty value among `keys` — used for aliases such as
    /// `manifests_dir`/`path` or `timeout_seconds`/`duration`.
    pub fn first_of(&self, keys: &[&str]) -> Option<&str> {
        keys.iter().find_map(|k| self.get(k))
    }
}

/// Public, serializable metadata about a registered provider.
///
/// Returned by `available_providers()` so the CLI/API can enumerate what is
/// installed without knowing any provider-specific types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInfo {
    pub scheme: String,
    pub description: String,
}

/// Builds a boxed provider from its parameters. Plain `fn` pointers (rather
/// than closures) keep the registry trivially `const`-friendly and cloneable.
pub type ProviderFactory = fn(&ProviderParams) -> Result<Box<dyn DiscoveryProvider>>;

struct ProviderEntry {
    scheme: &'static str,
    description: &'static str,
    factory: ProviderFactory,
}

/// Maps scheme strings to provider factories.
///
/// This is the single dispatch point shared by the CLI, the config loader, and
/// the REST API. Register once here and a provider is reachable everywhere.
#[derive(Default)]
pub struct ProviderRegistry {
    providers: BTreeMap<&'static str, ProviderEntry>,
}

impl ProviderRegistry {
    /// An empty registry (used in tests, and as the base for custom registries).
    pub fn new() -> Self {
        ProviderRegistry {
            providers: BTreeMap::new(),
        }
    }

    /// Register a provider factory under `scheme`. Re-registering the same
    /// scheme overwrites the earlier entry, which keeps `default_registry`
    /// declarative and idempotent.
    pub fn register(
        &mut self,
        scheme: &'static str,
        description: &'static str,
        factory: ProviderFactory,
    ) -> &mut Self {
        self.providers.insert(
            scheme,
            ProviderEntry {
                scheme,
                description,
                factory,
            },
        );
        self
    }

    /// Whether `scheme` has a provider registered.
    pub fn contains(&self, scheme: &str) -> bool {
        self.providers.contains_key(scheme)
    }

    /// All registered scheme names, sorted (deterministic output).
    pub fn schemes(&self) -> Vec<&'static str> {
        self.providers.keys().copied().collect()
    }

    /// Metadata for every registered provider, sorted by scheme.
    pub fn describe(&self) -> Vec<ProviderInfo> {
        self.providers
            .values()
            .map(|e| ProviderInfo {
                scheme: e.scheme.to_string(),
                description: e.description.to_string(),
            })
            .collect()
    }

    /// Construct the provider registered under `scheme`.
    ///
    /// Unknown schemes produce a clear error listing what *is* available —
    /// important because this message is what a user sees after a typo, and
    /// what a contributor sees when they forget to register their factory.
    pub fn create(
        &self,
        scheme: &str,
        params: &ProviderParams,
    ) -> Result<Box<dyn DiscoveryProvider>> {
        match self.providers.get(scheme) {
            Some(entry) => (entry.factory)(params),
            None => Err(Error::invalid_argument(format!(
                "unknown discovery scheme: '{scheme}' (available: {})",
                self.schemes().join(", ")
            ))),
        }
    }

    /// One-shot create-and-discover convenience used by the CLI/API.
    pub fn discover(&self, scheme: &str, params: &ProviderParams) -> Result<Graph> {
        self.create(scheme, params)?.discover()
    }
}

// ---------------------------------------------------------------------
// Provider factories (one per connector)
// ---------------------------------------------------------------------

fn terraform_provider(params: &ProviderParams) -> Result<Box<dyn DiscoveryProvider>> {
    Ok(Box::new(terraform::TerraformDiscoverer::new(
        params.get_or("path", ".").to_string(),
    )))
}

fn kubernetes_provider(params: &ProviderParams) -> Result<Box<dyn DiscoveryProvider>> {
    let dir = params
        .first_of(&["manifests_dir", "path"])
        .map(str::to_string);
    let mut discoverer = match dir {
        Some(dir) => kubernetes::KubernetesDiscoverer::from_manifests_dir(dir),
        None => kubernetes::KubernetesDiscoverer::new(),
    };
    if let Some(namespaces) = params.get_list("namespaces") {
        discoverer = discoverer.with_namespaces(namespaces);
    }
    Ok(Box::new(discoverer))
}

fn airflow_provider(params: &ProviderParams) -> Result<Box<dyn DiscoveryProvider>> {
    // `dag_folder` is the real input; `path` is accepted as an alias. `host`/
    // `port` are accepted for interface compatibility with older configs but
    // have no effect — this provider parses DAG files on disk, it does not talk
    // to the Airflow REST API (see src/discovery/airflow.rs).
    let folder = params
        .first_of(&["dag_folder", "path"])
        .unwrap_or(".")
        .to_string();
    Ok(Box::new(airflow::AirflowDiscoverer::new(folder)))
}

#[cfg(feature = "otlp-traces")]
fn otlp_provider(params: &ProviderParams) -> Result<Box<dyn DiscoveryProvider>> {
    let endpoint = params.get_or("endpoint", "0.0.0.0:4317").to_string();
    // `timeout_seconds` is the precise unit; `duration` (minutes) is kept for
    // interface compatibility with older configs/URIs.
    let mut discoverer = otlp::OtlpDiscoverer::new(endpoint);
    if params.get("timeout_seconds").is_some() {
        let seconds: u64 = params.get_parsed("timeout_seconds", 30u64)?;
        discoverer = discoverer.with_timeout(seconds);
    } else if params.get("duration").is_some() {
        let minutes: u64 = params.get_parsed("duration", 30u64)?;
        discoverer = discoverer.with_timeout(minutes.saturating_mul(60));
    }
    Ok(Box::new(discoverer))
}

/// The registry every entry point uses, with all built-in providers registered.
///
/// Adding a provider is a **one-line change here** plus its own module — this
/// function is the only place in the codebase that enumerates connectors.
pub fn default_registry() -> ProviderRegistry {
    let mut registry = ProviderRegistry::new();
    registry.register(
        "terraform",
        "Parse real .tf files via hcl-rs and derive dependency edges from actual resource references.",
        terraform_provider,
    );
    registry.register(
        "kubernetes",
        "Parse local Service/Deployment/StatefulSet/DaemonSet YAML manifests and infer edges from container env var references.",
        kubernetes_provider,
    );
    registry.register(
        "airflow",
        "Parse local Airflow DAG (.py) files: DAGs, tasks, task-dependency operators (>>/<<, set_downstream/upstream), and external system references.",
        airflow_provider,
    );
    #[cfg(feature = "otlp-traces")]
    registry.register(
        "otlp",
        "Bind a real OTLP/gRPC trace receiver and build edges from cross-service parent/child spans received during the collection window.",
        otlp_provider,
    );
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_exposes_the_builtin_schemes() {
        let registry = default_registry();
        assert!(registry.contains("terraform"));
        assert!(registry.contains("kubernetes"));
        assert!(registry.contains("airflow"));
        #[cfg(feature = "otlp-traces")]
        assert!(registry.contains("otlp"));
    }

    #[test]
    fn schemes_are_sorted_for_deterministic_output() {
        let schemes = default_registry().schemes();
        let mut sorted = schemes.clone();
        sorted.sort_unstable();
        assert_eq!(schemes, sorted);
    }

    #[test]
    fn describe_returns_metadata_for_every_registered_scheme() {
        let registry = default_registry();
        let described = registry.describe();
        assert_eq!(described.len(), registry.schemes().len());
        for info in &described {
            assert!(!info.description.is_empty());
        }
    }

    #[test]
    fn unknown_scheme_is_a_clear_error_not_a_panic() {
        let registry = default_registry();
        let err = match registry.create("not-a-real-source", &ProviderParams::new()) {
            Ok(_) => panic!("expected an error for an unknown scheme"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(msg.contains("unknown discovery scheme"));
        assert!(msg.contains("terraform")); // lists what is available
    }

    #[test]
    fn create_terraform_provider_from_params() {
        let registry = default_registry();
        let params = ProviderParams::new().with("path", "/tmp/does-not-exist");
        let provider = registry.create("terraform", &params).unwrap();
        assert_eq!(provider.scheme(), "terraform");
        // Missing directory is an honest empty graph, not an error.
        assert_eq!(provider.discover().unwrap().node_count(), 0);
    }

    #[test]
    fn provider_params_parse_lists_and_numbers() {
        let params =
            ProviderParams::from_pairs([("namespaces", "a;b, c"), ("timeout_seconds", "7")]);
        assert_eq!(
            params.get_list("namespaces"),
            Some(vec!["a".into(), "b".into(), "c".into()])
        );
        assert_eq!(params.get_parsed("timeout_seconds", 30u64).unwrap(), 7);
        assert_eq!(params.get_parsed("missing", 30u64).unwrap(), 30);
        assert!(params.get_parsed::<u64>("timeout_seconds", 0).is_ok());
    }

    #[test]
    fn provider_params_rejects_malformed_numbers() {
        let params = ProviderParams::new().with("timeout_seconds", "not-a-number");
        assert!(params.get_parsed("timeout_seconds", 30u64).is_err());
    }

    #[test]
    fn provider_params_first_of_returns_first_present_alias() {
        let params = ProviderParams::new().with("path", "/x");
        assert_eq!(params.first_of(&["manifests_dir", "path"]), Some("/x"));
        assert_eq!(params.first_of(&["absent", "missing"]), None);
    }
}
