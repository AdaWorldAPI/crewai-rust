//! Capability Registry — global registry for resolving capability identifiers.
//!
//! The registry loads capabilities from:
//! 1. Built-in capabilities (compiled into the binary)
//! 2. YAML files in the `capabilities/` directory
//! 3. Programmatically registered capabilities
//!
//! Resolution is by namespaced ID: `registry.resolve("minecraft:server_control")`

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::capability::Capability;

/// Global capability registry.
///
/// Holds all known capabilities indexed by their namespaced ID.
/// Supports loading from filesystem directories, individual files,
/// and programmatic registration.
#[derive(Debug, Default)]
pub struct CapabilityRegistry {
    /// Capabilities indexed by ID
    capabilities: HashMap<String, Capability>,

    /// Search paths for capability YAML files
    search_paths: Vec<PathBuf>,

    /// Namespace aliases (e.g., "ms" -> "o365")
    aliases: HashMap<String, String>,
}

impl CapabilityRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with default search paths.
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        // Standard search paths
        reg.search_paths.push(PathBuf::from("capabilities"));
        reg.search_paths
            .push(PathBuf::from("~/.crewai/capabilities"));
        reg.search_paths
            .push(PathBuf::from("/etc/crewai/capabilities"));
        reg
    }

    /// Add a search path for capability YAML files.
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Register a namespace alias.
    pub fn add_alias(&mut self, alias: &str, target_namespace: &str) {
        self.aliases
            .insert(alias.to_string(), target_namespace.to_string());
    }

    /// Register a capability directly.
    pub fn register(&mut self, capability: Capability) {
        self.capabilities
            .insert(capability.id.clone(), capability);
    }

    /// Register multiple capabilities from a YAML file.
    /// The file can contain a single `capability:` or a `capabilities:` list.
    pub fn register_from_file(&mut self, path: &str) -> Result<usize, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;

        // Try single capability first
        if let Ok(cap) = Capability::from_yaml(&content) {
            let id = cap.id.clone();
            self.capabilities.insert(id, cap);
            return Ok(1);
        }

        // Try list of capabilities
        let list: CapabilityListWrapper = serde_yaml::from_str(&content)?;
        let count = list.capabilities.len();
        for cap in list.capabilities {
            self.capabilities.insert(cap.id.clone(), cap);
        }
        Ok(count)
    }

    /// Load all capability YAML files from a directory (recursive).
    pub fn load_directory(&mut self, dir: &Path) -> Result<usize, Box<dyn std::error::Error>> {
        let mut count = 0;
        if !dir.exists() {
            return Ok(0);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                count += self.load_directory(&path)?;
            } else if path.extension().map_or(false, |ext| {
                ext == "yaml" || ext == "yml"
            }) {
                match self.register_from_file(path.to_str().unwrap_or_default()) {
                    Ok(n) => count += n,
                    Err(e) => {
                        log::warn!(
                            "Failed to load capability from {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load capabilities from all registered search paths.
    pub fn load_all(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut count = 0;
        let paths = self.search_paths.clone();
        for path in &paths {
            count += self.load_directory(path)?;
        }
        Ok(count)
    }

    /// Resolve a capability by its namespaced ID.
    ///
    /// Supports alias resolution: if the namespace is an alias, it's resolved
    /// before lookup. If the capability isn't in memory, searches the filesystem.
    pub fn resolve(&mut self, id: &str) -> Option<&Capability> {
        // Apply alias resolution
        let resolved_id = self.resolve_alias(id);

        // Check in-memory first
        if self.capabilities.contains_key(&resolved_id) {
            return self.capabilities.get(&resolved_id);
        }

        // Try filesystem search
        let namespace = resolved_id.split(':').next().unwrap_or_default();
        let name = resolved_id.split(':').nth(1).unwrap_or_default();

        for search_path in &self.search_paths.clone() {
            let candidate = search_path.join(namespace).join(format!("{}.yaml", name));
            if candidate.exists() {
                if let Ok(_) = self.register_from_file(candidate.to_str().unwrap_or_default()) {
                    return self.capabilities.get(&resolved_id);
                }
            }

            let candidate_yml = search_path.join(namespace).join(format!("{}.yml", name));
            if candidate_yml.exists() {
                if let Ok(_) = self.register_from_file(candidate_yml.to_str().unwrap_or_default())
                {
                    return self.capabilities.get(&resolved_id);
                }
            }
        }

        None
    }

    /// Resolve a list of capability IDs (e.g., from an agent card).
    /// Returns (resolved, unresolved) tuples.
    pub fn resolve_many(&mut self, ids: &[String]) -> (Vec<&Capability>, Vec<String>) {
        let mut resolved = Vec::new();
        let mut unresolved = Vec::new();

        // We need to work around borrow checker — resolve each and collect IDs
        let resolved_ids: Vec<String> = ids
            .iter()
            .filter_map(|id| {
                let rid = self.resolve_alias(id);
                if self.capabilities.contains_key(&rid) {
                    Some(rid)
                } else {
                    None
                }
            })
            .collect();

        // Try filesystem for the rest
        for id in ids {
            let rid = self.resolve_alias(id);
            if !resolved_ids.contains(&rid) {
                let namespace = rid.split(':').next().unwrap_or_default();
                let name = rid.split(':').nth(1).unwrap_or_default();

                let mut found = false;
                for search_path in &self.search_paths.clone() {
                    let candidate =
                        search_path.join(namespace).join(format!("{}.yaml", name));
                    if candidate.exists() {
                        if self
                            .register_from_file(candidate.to_str().unwrap_or_default())
                            .is_ok()
                        {
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    unresolved.push(id.clone());
                }
            }
        }

        // Now collect all resolved references
        for id in ids {
            let rid = self.resolve_alias(id);
            if let Some(cap) = self.capabilities.get(&rid) {
                // SAFETY: we're returning references to HashMap values that won't move
                // because we don't modify the map after this point
                resolved.push(unsafe { &*(cap as *const Capability) });
            }
        }

        (resolved, unresolved)
    }

    /// List all registered capabilities.
    pub fn list(&self) -> Vec<&Capability> {
        self.capabilities.values().collect()
    }

    /// List capabilities by namespace.
    pub fn list_by_namespace(&self, namespace: &str) -> Vec<&Capability> {
        let resolved_ns = self.aliases.get(namespace).cloned().unwrap_or_else(|| namespace.to_string());
        self.capabilities
            .values()
            .filter(|c| c.namespace() == resolved_ns)
            .collect()
    }

    /// Search capabilities by tag.
    pub fn search_by_tag(&self, tag: &str) -> Vec<&Capability> {
        self.capabilities
            .values()
            .filter(|c| c.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Search capabilities by description (substring match).
    pub fn search_by_description(&self, query: &str) -> Vec<&Capability> {
        let query_lower = query.to_lowercase();
        self.capabilities
            .values()
            .filter(|c| c.description.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Get the total number of registered capabilities.
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }

    /// Resolve aliases in an ID
    fn resolve_alias(&self, id: &str) -> String {
        let parts: Vec<&str> = id.splitn(2, ':').collect();
        if parts.len() == 2 {
            let namespace = parts[0];
            let name = parts[1];
            if let Some(target) = self.aliases.get(namespace) {
                return format!("{}:{}", target, name);
            }
        }
        id.to_string()
    }
}

/// Wrapper for YAML list of capabilities
#[derive(Debug, serde::Deserialize)]
struct CapabilityListWrapper {
    capabilities: Vec<Capability>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::capability::{CapabilityInterface, InterfaceProtocol};

    #[test]
    fn test_register_and_resolve() {
        let mut registry = CapabilityRegistry::new();

        let cap = Capability {
            id: "test:hello".to_string(),
            version: "1.0.0".to_string(),
            description: "A test capability".to_string(),
            tags: vec!["test".to_string()],
            metadata: Default::default(),
            interface: CapabilityInterface {
                protocol: InterfaceProtocol::Native,
                config: Default::default(),
                endpoint_template: None,
                auth_scheme: None,
            },
            tools: vec![],
            policy: Default::default(),
            depends_on: vec![],
            cam_opcode_range: None,
        };

        registry.register(cap);

        assert_eq!(registry.len(), 1);
        let resolved = registry.resolve("test:hello");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().id, "test:hello");
    }

    #[test]
    fn test_alias_resolution() {
        let mut registry = CapabilityRegistry::new();
        registry.add_alias("ms", "o365");

        let cap = Capability {
            id: "o365:mail".to_string(),
            version: "1.0.0".to_string(),
            description: "Mail capability".to_string(),
            tags: vec![],
            metadata: Default::default(),
            interface: CapabilityInterface {
                protocol: InterfaceProtocol::MsGraph,
                config: Default::default(),
                endpoint_template: None,
                auth_scheme: None,
            },
            tools: vec![],
            policy: Default::default(),
            depends_on: vec![],
            cam_opcode_range: None,
        };

        registry.register(cap);

        // Should resolve via alias
        let resolved = registry.resolve("ms:mail");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().id, "o365:mail");
    }

    #[test]
    fn test_search_by_tag() {
        let mut registry = CapabilityRegistry::new();

        let make_cap = |id: &str, tags: Vec<&str>| Capability {
            id: id.to_string(),
            version: "1.0.0".to_string(),
            description: format!("{} capability", id),
            tags: tags.into_iter().map(String::from).collect(),
            metadata: Default::default(),
            interface: CapabilityInterface {
                protocol: InterfaceProtocol::Native,
                config: Default::default(),
                endpoint_template: None,
                auth_scheme: None,
            },
            tools: vec![],
            policy: Default::default(),
            depends_on: vec![],
            cam_opcode_range: None,
        };

        registry.register(make_cap("game:mc", vec!["gaming", "server"]));
        registry.register(make_cap("game:cs", vec!["gaming", "server"]));
        registry.register(make_cap("cloud:aws", vec!["cloud"]));

        let gaming = registry.search_by_tag("gaming");
        assert_eq!(gaming.len(), 2);

        let cloud = registry.search_by_tag("cloud");
        assert_eq!(cloud.len(), 1);
    }
}
