use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VfsError {
    #[error("path not allowed: {0}")]
    PathNotAllowed(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A directory mapping from guest path to host path with permissions.
#[derive(Debug, Clone)]
pub struct DirMount {
    pub guest_path: String,
    pub host_path: PathBuf,
    pub writable: bool,
}

impl DirMount {
    pub fn new(guest: impl Into<String>, host: impl Into<PathBuf>, writable: bool) -> Self {
        Self {
            guest_path: guest.into(),
            host_path: host.into(),
            writable,
        }
    }

    pub fn read_only(guest: impl Into<String>, host: impl Into<PathBuf>) -> Self {
        Self::new(guest, host, false)
    }

    pub fn read_write(guest: impl Into<String>, host: impl Into<PathBuf>) -> Self {
        Self::new(guest, host, true)
    }
}

/// Filesystem configuration for a WASI instance.
#[derive(Debug, Clone, Default)]
pub struct FsConfig {
    pub mounts: Vec<DirMount>,
    pub preopens: Vec<DirMount>,
}

impl FsConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a preopened directory (accessible to the WASI program).
    pub fn preopen(mut self, mount: DirMount) -> Self {
        self.preopens.push(mount);
        self
    }

    /// Parse a mount spec like `/host/path:/guest/path:ro`
    pub fn parse_mount(spec: &str) -> Result<DirMount, VfsError> {
        let parts: Vec<&str> = spec.splitn(3, ':').collect();
        match parts.as_slice() {
            [host, guest] => {
                let host_path = Path::new(host).to_path_buf();
                Ok(DirMount::new(*guest, host_path, false))
            }
            [host, guest, mode] => {
                let host_path = Path::new(host).to_path_buf();
                let writable = *mode == "rw";
                Ok(DirMount::new(*guest, host_path, writable))
            }
            _ => Err(VfsError::PathNotAllowed(PathBuf::from(spec))),
        }
    }
}
