use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Sandbox policy — deny by default, explicit grants only.
///
/// ```toml
/// [sandbox]
/// max_memory = "256MB"
/// max_time = "30s"
/// allow_env = ["PATH", "HOME"]
///
/// [[sandbox.fs]]
/// guest = "/data"
/// host = "/tmp/sandbox-data"
/// permission = "ro"
///
/// [[sandbox.fs]]
/// guest = "/out"
/// host = "/tmp/sandbox-out"
/// permission = "rw"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxPolicy {
    /// Maximum linear memory in bytes. Default: 256MB.
    #[serde(
        serialize_with = "serialize_bytes",
        deserialize_with = "deserialize_bytes"
    )]
    pub max_memory: usize,

    /// Maximum execution time. Default: 30s.
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub max_time: Duration,

    /// Maximum table elements. Default: 10_000.
    pub max_table_elements: usize,

    /// Maximum number of WASM instances. Default: 1.
    pub max_instances: usize,

    /// Allowed environment variables. Empty = none passed through.
    pub allow_env: Vec<String>,

    /// Filesystem mounts. Empty = no filesystem access.
    pub fs: Vec<FsMount>,

    /// Allow outbound TCP connections. Default: false.
    pub allow_tcp: bool,

    /// Allow outbound UDP sockets. Default: false.
    pub allow_udp: bool,

    /// Allow DNS resolution. Default: false.
    pub allow_dns: bool,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            max_memory: 256 * 1024 * 1024, // 256MB
            max_time: Duration::from_secs(30),
            max_table_elements: 10_000,
            max_instances: 1,
            allow_env: Vec::new(),
            fs: Vec::new(),
            allow_tcp: false,
            allow_udp: false,
            allow_dns: false,
        }
    }
}

impl SandboxPolicy {
    /// Load policy from a TOML string.
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        #[derive(Deserialize)]
        struct Wrapper {
            sandbox: SandboxPolicy,
        }
        let wrapper: Wrapper = toml::from_str(s)?;
        Ok(wrapper.sandbox)
    }

    /// Load policy from a TOML file.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, PolicyError> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_toml(&content)?)
    }

    /// Filter environment variables through the allow list.
    /// Only variables named in `allow_env` are kept.
    pub fn filter_env<'a>(
        &self,
        vars: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Vec<(String, String)> {
        vars.into_iter()
            .filter(|(key, _)| self.allow_env.iter().any(|allowed| allowed == key))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
}

/// A filesystem mount in the sandbox policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsMount {
    /// Guest path (what the WASM program sees).
    pub guest: String,
    /// Host path (actual directory on the host).
    pub host: PathBuf,
    /// Permission level.
    pub permission: FsPermission,
}

/// Filesystem permission level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FsPermission {
    /// Read-only access.
    Ro,
    /// Read-write access.
    Rw,
}

impl FsPermission {
    pub fn is_writable(self) -> bool {
        matches!(self, FsPermission::Rw)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

// --- serde helpers for human-readable byte sizes ---

fn serialize_bytes<S: serde::Serializer>(bytes: &usize, s: S) -> Result<S::Ok, S::Error> {
    let val = *bytes;
    let human = if val % (1024 * 1024 * 1024) == 0 {
        format!("{}GB", val / (1024 * 1024 * 1024))
    } else if val % (1024 * 1024) == 0 {
        format!("{}MB", val / (1024 * 1024))
    } else if val % 1024 == 0 {
        format!("{}KB", val / 1024)
    } else {
        format!("{}B", val)
    };
    s.serialize_str(&human)
}

fn deserialize_bytes<'de, D: serde::Deserializer<'de>>(d: D) -> Result<usize, D::Error> {
    let s = String::deserialize(d)?;
    parse_bytes(&s).map_err(serde::de::Error::custom)
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

// --- serde helpers for human-readable durations ---

fn serialize_duration<S: serde::Serializer>(dur: &Duration, s: S) -> Result<S::Ok, S::Error> {
    let secs = dur.as_secs();
    let human = if secs % 3600 == 0 && secs > 0 {
        format!("{}h", secs / 3600)
    } else if secs % 60 == 0 && secs > 0 {
        format!("{}m", secs / 60)
    } else {
        format!("{}s", secs)
    };
    s.serialize_str(&human)
}

fn deserialize_duration<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
    let s = String::deserialize(d)?;
    parse_duration(&s).map_err(serde::de::Error::custom)
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix('h') {
        n.trim()
            .parse::<u64>()
            .map(|n| Duration::from_secs(n * 3600))
    } else if let Some(n) = s.strip_suffix('m') {
        n.trim()
            .parse::<u64>()
            .map(|n| Duration::from_secs(n * 60))
    } else if let Some(n) = s.strip_suffix('s') {
        n.trim().parse::<u64>().map(Duration::from_secs)
    } else if let Some(n) = s.strip_suffix("ms") {
        n.trim().parse::<u64>().map(Duration::from_millis)
    } else {
        s.parse::<u64>().map(Duration::from_secs)
    }
    .map_err(|_| format!("invalid duration: {s}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_denies_everything() {
        let policy = SandboxPolicy::default();
        assert_eq!(policy.max_memory, 256 * 1024 * 1024);
        assert_eq!(policy.max_time, Duration::from_secs(30));
        assert!(policy.fs.is_empty());
        assert!(policy.allow_env.is_empty());
        assert!(!policy.allow_tcp);
        assert!(!policy.allow_udp);
        assert!(!policy.allow_dns);
    }

    #[test]
    fn parse_toml_policy() {
        let toml = r#"
            [sandbox]
            max_memory = "128MB"
            max_time = "10s"
            allow_env = ["PATH", "HOME"]

            [[sandbox.fs]]
            guest = "/data"
            host = "/tmp/data"
            permission = "ro"

            [[sandbox.fs]]
            guest = "/out"
            host = "/tmp/out"
            permission = "rw"
        "#;
        let policy = SandboxPolicy::from_toml(toml).unwrap();
        assert_eq!(policy.max_memory, 128 * 1024 * 1024);
        assert_eq!(policy.max_time, Duration::from_secs(10));
        assert_eq!(policy.allow_env, vec!["PATH", "HOME"]);
        assert_eq!(policy.fs.len(), 2);
        assert_eq!(policy.fs[0].permission, FsPermission::Ro);
        assert_eq!(policy.fs[1].permission, FsPermission::Rw);
        // Networking defaults to false when not specified
        assert!(!policy.allow_tcp);
        assert!(!policy.allow_udp);
        assert!(!policy.allow_dns);
    }

    #[test]
    fn parse_toml_with_networking() {
        let toml = r#"
            [sandbox]
            max_memory = "64MB"
            max_time = "5s"
            allow_tcp = true
            allow_udp = false
            allow_dns = true
        "#;
        let policy = SandboxPolicy::from_toml(toml).unwrap();
        assert!(policy.allow_tcp);
        assert!(!policy.allow_udp);
        assert!(policy.allow_dns);
        assert_eq!(policy.max_memory, 64 * 1024 * 1024);
    }

    #[test]
    fn filter_env() {
        let policy = SandboxPolicy {
            allow_env: vec!["PATH".into(), "HOME".into()],
            ..Default::default()
        };
        let vars = vec![
            ("PATH", "/usr/bin"),
            ("HOME", "/home/user"),
            ("SECRET", "leaked"),
        ];
        let filtered = policy.filter_env(vars);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|(k, _)| k != "SECRET"));
    }
}
