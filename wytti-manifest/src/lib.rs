use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use wytti_sandbox::{FsMount, FsPermission, SandboxPolicy};

/// A `.fytti.toml` manifest declaring what a WASM app needs from its host.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub app: AppInfo,
    #[serde(default)]
    pub capabilities: Capabilities,
}

/// App metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct AppInfo {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
}

/// Capability declarations.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Capabilities {
    /// Can use TCP/UDP/DNS.
    pub network: bool,
    /// Can access preopened dirs.
    pub filesystem: bool,
    /// Can read/write clipboard.
    pub clipboard: bool,
    /// Memory limit (e.g. "128MB").
    #[serde(default, deserialize_with = "deserialize_opt_bytes")]
    pub max_memory: Option<usize>,
    /// Execution time limit (e.g. "30s").
    #[serde(default, deserialize_with = "deserialize_opt_duration")]
    pub max_time: Option<Duration>,
    /// Filesystem mount declarations.
    #[serde(default)]
    pub fs: Vec<ManifestFsMount>,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            network: false,
            filesystem: false,
            clipboard: false,
            max_memory: None,
            max_time: None,
            fs: Vec::new(),
        }
    }
}

/// A filesystem mount declared in the manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestFsMount {
    pub guest: String,
    pub host: PathBuf,
    pub permission: FsPermission,
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

const MANIFEST_FILENAME: &str = ".fytti.toml";

impl Manifest {
    /// Parse a manifest from a TOML string.
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    /// Load a manifest from a file path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_toml(&content)?)
    }

    /// Discover a `.fytti.toml` manifest next to a .wasm file.
    ///
    /// Given `/some/path/app.wasm`, looks for `/some/path/.fytti.toml`.
    /// Returns `Ok(None)` if no manifest exists.
    pub fn discover(wasm_path: impl AsRef<Path>) -> Result<Option<Self>, ManifestError> {
        let wasm_path = wasm_path.as_ref();
        let dir = wasm_path.parent().unwrap_or(Path::new("."));
        let manifest_path = dir.join(MANIFEST_FILENAME);
        if manifest_path.exists() {
            Ok(Some(Self::from_file(&manifest_path)?))
        } else {
            Ok(None)
        }
    }

    /// Convert this manifest's capabilities into a `SandboxPolicy`.
    pub fn to_sandbox_policy(&self) -> SandboxPolicy {
        let caps = &self.capabilities;

        let fs: Vec<FsMount> = caps
            .fs
            .iter()
            .map(|m| FsMount {
                guest: m.guest.clone(),
                host: m.host.clone(),
                permission: m.permission,
            })
            .collect();

        SandboxPolicy {
            max_memory: caps.max_memory.unwrap_or(256 * 1024 * 1024),
            max_time: caps.max_time.unwrap_or(Duration::from_secs(30)),
            fs,
            allow_tcp: caps.network,
            allow_udp: caps.network,
            allow_dns: caps.network,
            ..Default::default()
        }
    }
}

// --- serde helpers ---

fn deserialize_opt_bytes<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<usize>, D::Error> {
    let opt: Option<String> = Option::deserialize(d)?;
    match opt {
        None => Ok(None),
        Some(s) => parse_bytes(&s).map(Some).map_err(serde::de::Error::custom),
    }
}

fn parse_bytes(s: &str) -> Result<usize, String> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix("GB") {
        n.trim().parse::<usize>().map(|n| n * 1024 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("MB") {
        n.trim().parse::<usize>().map(|n| n * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("KB") {
        n.trim().parse::<usize>().map(|n| n * 1024)
    } else if let Some(n) = s.strip_suffix('B') {
        n.trim().parse::<usize>()
    } else {
        s.parse::<usize>()
    }
    .map_err(|_| format!("invalid byte size: {s}"))
}

fn deserialize_opt_duration<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Duration>, D::Error> {
    let opt: Option<String> = Option::deserialize(d)?;
    match opt {
        None => Ok(None),
        Some(s) => parse_duration(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix("ms") {
        n.trim().parse::<u64>().map(Duration::from_millis)
    } else if let Some(n) = s.strip_suffix('h') {
        n.trim()
            .parse::<u64>()
            .map(|n| Duration::from_secs(n * 3600))
    } else if let Some(n) = s.strip_suffix('m') {
        n.trim()
            .parse::<u64>()
            .map(|n| Duration::from_secs(n * 60))
    } else if let Some(n) = s.strip_suffix('s') {
        n.trim().parse::<u64>().map(Duration::from_secs)
    } else {
        s.parse::<u64>().map(Duration::from_secs)
    }
    .map_err(|_| format!("invalid duration: {s}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_full_manifest() {
        let toml = r#"
            [app]
            name = "My App"
            version = "0.1.0"
            description = "A cool WASM app"
            author = "Someone"

            [capabilities]
            network = false
            filesystem = false
            clipboard = false
            max_memory = "128MB"
            max_time = "30s"

            [[capabilities.fs]]
            guest = "/data"
            host = "/tmp/app-data"
            permission = "ro"
        "#;
        let manifest = Manifest::from_toml(toml).unwrap();
        assert_eq!(manifest.app.name, "My App");
        assert_eq!(manifest.app.version.as_deref(), Some("0.1.0"));
        assert_eq!(manifest.app.description.as_deref(), Some("A cool WASM app"));
        assert_eq!(manifest.app.author.as_deref(), Some("Someone"));
        assert!(!manifest.capabilities.network);
        assert!(!manifest.capabilities.filesystem);
        assert!(!manifest.capabilities.clipboard);
        assert_eq!(manifest.capabilities.max_memory, Some(128 * 1024 * 1024));
        assert_eq!(manifest.capabilities.max_time, Some(Duration::from_secs(30)));
        assert_eq!(manifest.capabilities.fs.len(), 1);
        assert_eq!(manifest.capabilities.fs[0].guest, "/data");
        assert_eq!(manifest.capabilities.fs[0].permission, FsPermission::Ro);
    }

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
            [app]
            name = "Minimal"
        "#;
        let manifest = Manifest::from_toml(toml).unwrap();
        assert_eq!(manifest.app.name, "Minimal");
        assert!(manifest.app.version.is_none());
        assert!(manifest.app.description.is_none());
        assert!(!manifest.capabilities.network);
        assert!(manifest.capabilities.fs.is_empty());
        assert!(manifest.capabilities.max_memory.is_none());
        assert!(manifest.capabilities.max_time.is_none());
    }

    #[test]
    fn convert_to_sandbox_policy() {
        let toml = r#"
            [app]
            name = "Networked App"

            [capabilities]
            network = true
            max_memory = "64MB"
            max_time = "10s"

            [[capabilities.fs]]
            guest = "/out"
            host = "/tmp/out"
            permission = "rw"
        "#;
        let manifest = Manifest::from_toml(toml).unwrap();
        let policy = manifest.to_sandbox_policy();

        assert_eq!(policy.max_memory, 64 * 1024 * 1024);
        assert_eq!(policy.max_time, Duration::from_secs(10));
        assert!(policy.allow_tcp);
        assert!(policy.allow_udp);
        assert!(policy.allow_dns);
        assert_eq!(policy.fs.len(), 1);
        assert_eq!(policy.fs[0].guest, "/out");
        assert_eq!(policy.fs[0].permission, FsPermission::Rw);
    }

    #[test]
    fn convert_minimal_to_sandbox_policy() {
        let toml = r#"
            [app]
            name = "Minimal"
        "#;
        let manifest = Manifest::from_toml(toml).unwrap();
        let policy = manifest.to_sandbox_policy();

        // Should use defaults
        assert_eq!(policy.max_memory, 256 * 1024 * 1024);
        assert_eq!(policy.max_time, Duration::from_secs(30));
        assert!(!policy.allow_tcp);
        assert!(policy.fs.is_empty());
    }

    #[test]
    fn discover_manifest_from_wasm_path() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = dir.path().join(MANIFEST_FILENAME);
        let wasm_path = dir.path().join("app.wasm");

        // No manifest yet
        assert!(Manifest::discover(&wasm_path).unwrap().is_none());

        // Write manifest
        let mut f = std::fs::File::create(&manifest_path).unwrap();
        write!(f, "[app]\nname = \"Discovered\"").unwrap();
        drop(f);

        let manifest = Manifest::discover(&wasm_path).unwrap().unwrap();
        assert_eq!(manifest.app.name, "Discovered");
    }

    #[test]
    fn from_file_works() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.fytti.toml");
        std::fs::write(
            &path,
            "[app]\nname = \"FromFile\"\nversion = \"1.0.0\"",
        )
        .unwrap();

        let manifest = Manifest::from_file(&path).unwrap();
        assert_eq!(manifest.app.name, "FromFile");
        assert_eq!(manifest.app.version.as_deref(), Some("1.0.0"));
    }
}
