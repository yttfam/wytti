use anyhow::Result;
use clap::{Parser, Subcommand};
use wytti_manifest::Manifest;
use wytti_runtime::{Runtime, RuntimeConfig};
use wytti_sandbox::SandboxPolicy;
use wytti_vfs::FsConfig;

#[derive(Parser)]
#[command(name = "wytti", about = "WASI runtime — run WASM binaries, sandboxed")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a WASM binary
    Run {
        /// Path to the .wasm file
        file: String,

        /// Arguments to pass to the WASM program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// Mount a directory: /host/path:/guest/path[:ro|rw]
        #[arg(short = 'd', long = "dir")]
        dirs: Vec<String>,

        /// Set environment variable: KEY=VALUE
        #[arg(short, long = "env")]
        envs: Vec<String>,

        /// Sandbox policy file (TOML)
        #[arg(short, long)]
        policy: Option<String>,

        /// Maximum memory (e.g. 128MB, 1GB)
        #[arg(long)]
        max_memory: Option<String>,

        /// Maximum execution time (e.g. 10s, 5m)
        #[arg(long)]
        max_time: Option<String>,

        /// Allow outbound TCP connections
        #[arg(long)]
        allow_tcp: bool,

        /// Allow outbound UDP sockets
        #[arg(long)]
        allow_udp: bool,

        /// Allow DNS resolution
        #[arg(long)]
        allow_dns: bool,

        /// Allow all networking (TCP + UDP + DNS)
        #[arg(long, short = 'N')]
        allow_net: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Run {
            file,
            args,
            dirs,
            envs,
            policy,
            max_memory,
            max_time,
            allow_tcp,
            allow_udp,
            allow_dns,
            allow_net,
        } => {
            // Load sandbox policy: explicit --policy > .fytti.toml manifest > default
            let mut sandbox = if let Some(ref path) = policy {
                SandboxPolicy::from_file(path)?
            } else if let Some(manifest) = Manifest::discover(&file)? {
                manifest.to_sandbox_policy()
            } else {
                SandboxPolicy::default()
            };

            // CLI overrides
            if let Some(ref mem) = max_memory {
                sandbox.max_memory = parse_bytes(mem)?;
            }
            if let Some(ref time) = max_time {
                sandbox.max_time = parse_duration(time)?;
            }
            if allow_net || allow_tcp {
                sandbox.allow_tcp = true;
            }
            if allow_net || allow_udp {
                sandbox.allow_udp = true;
            }
            if allow_net || allow_dns {
                sandbox.allow_dns = true;
            }

            // Build filesystem config from CLI --dir flags
            let mut fs_config = FsConfig::new();
            for dir_spec in &dirs {
                let mount = FsConfig::parse_mount(dir_spec)?;
                fs_config = fs_config.preopen(mount);
            }

            // Build runtime config
            let mut config = RuntimeConfig::new()
                .args(std::iter::once(file.clone()).chain(args))
                .fs(fs_config)
                .sandbox(sandbox);

            for env_spec in &envs {
                if let Some((key, value)) = env_spec.split_once('=') {
                    config = config.env(key, value);
                }
            }

            let runtime = Runtime::new()?;
            runtime.run_file(&file, &config)?;
        }
    }

    Ok(())
}

fn parse_bytes(s: &str) -> Result<usize> {
    let s = s.trim();
    let result = if let Some(n) = s.strip_suffix("GB") {
        n.trim().parse::<usize>().map(|n| n * 1024 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("MB") {
        n.trim().parse::<usize>().map(|n| n * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("KB") {
        n.trim().parse::<usize>().map(|n| n * 1024)
    } else {
        s.parse::<usize>()
    };
    result.map_err(|_| anyhow::anyhow!("invalid byte size: {s}"))
}

fn parse_duration(s: &str) -> Result<std::time::Duration> {
    let s = s.trim();
    let result = if let Some(n) = s.strip_suffix('h') {
        n.trim()
            .parse::<u64>()
            .map(|n| std::time::Duration::from_secs(n * 3600))
    } else if let Some(n) = s.strip_suffix('m') {
        n.trim()
            .parse::<u64>()
            .map(|n| std::time::Duration::from_secs(n * 60))
    } else if let Some(n) = s.strip_suffix('s') {
        n.trim()
            .parse::<u64>()
            .map(std::time::Duration::from_secs)
    } else {
        s.parse::<u64>().map(std::time::Duration::from_secs)
    };
    result.map_err(|_| anyhow::anyhow!("invalid duration: {s}"))
}
