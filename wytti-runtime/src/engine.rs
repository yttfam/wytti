use std::sync::Arc;
use std::time::Duration;
use wasmtime::{Engine, Module, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::{DirPerms, FilePerms, IoView, WasiCtx, WasiCtxBuilder, WasiView};
use wytti_sandbox::{FsPermission, SandboxPolicy};
use wytti_vfs::FsConfig;

use crate::RuntimeError;

/// Configuration for a WASI runtime instance.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub fs: FsConfig,
    pub inherit_stdio: bool,
    pub sandbox: SandboxPolicy,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            args: Vec::new(),
            env: Vec::new(),
            fs: FsConfig::default(),
            inherit_stdio: true,
            sandbox: SandboxPolicy::default(),
        }
    }
}

impl RuntimeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn fs(mut self, fs: FsConfig) -> Self {
        self.fs = fs;
        self
    }

    pub fn inherit_stdio(mut self, inherit: bool) -> Self {
        self.inherit_stdio = inherit;
        self
    }

    pub fn sandbox(mut self, sandbox: SandboxPolicy) -> Self {
        self.sandbox = sandbox;
        self
    }
}

/// Store state for P1 (core module) execution.
struct P1State {
    wasi: WasiP1Ctx,
    limits: StoreLimits,
}

/// Store state for P2 (component) execution.
struct P2State {
    ctx: WasiCtx,
    table: wasmtime::component::ResourceTable,
    limits: StoreLimits,
}

impl IoView for P2State {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.table
    }
}

impl WasiView for P2State {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

/// The Wytti WASI runtime.
pub struct Runtime {
    engine: Arc<Engine>,
}

impl Runtime {
    pub fn new() -> Result<Self, RuntimeError> {
        let mut config = wasmtime::Config::new();
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(RuntimeError::Load)?;
        Ok(Self {
            engine: Arc::new(engine),
        })
    }

    /// Run a WASM binary from a file path. Auto-detects P1 (core module) vs P2 (component).
    pub fn run_file(
        &self,
        path: impl AsRef<std::path::Path>,
        config: &RuntimeConfig,
    ) -> Result<(), RuntimeError> {
        let bytes = std::fs::read(path.as_ref()).map_err(|e| RuntimeError::Load(e.into()))?;
        self.run(&bytes, config)
    }

    /// Run a WASM binary from bytes. Auto-detects P1 vs P2.
    pub fn run(&self, wasm: &[u8], config: &RuntimeConfig) -> Result<(), RuntimeError> {
        if is_component(wasm) {
            self.run_component(wasm, config)
        } else {
            self.run_core_module(wasm, config)
        }
    }

    /// Run a WASI P1 core module (has `_start` export).
    fn run_core_module(&self, wasm: &[u8], config: &RuntimeConfig) -> Result<(), RuntimeError> {
        let module = Module::new(&self.engine, wasm).map_err(RuntimeError::Load)?;

        let mut linker = wasmtime::Linker::new(&self.engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |state: &mut P1State| {
            &mut state.wasi
        })
        .map_err(RuntimeError::Instantiate)?;

        let limits = self.build_limits(config);
        let wasi = self.build_wasi_ctx_p1(config);
        let state = P1State { wasi, limits };
        let mut store = Store::new(&self.engine, state);

        store.limiter(|state| &mut state.limits);
        self.configure_epoch(&mut store, config);

        let _ticker = self.spawn_epoch_ticker(config);

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(RuntimeError::Instantiate)?;

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|_| RuntimeError::NoEntryPoint)?;

        self.handle_result(start.call(&mut store, ()), config)
    }

    /// Run a WASI P2 component (component model with `wasi:cli/run`).
    fn run_component(&self, wasm: &[u8], config: &RuntimeConfig) -> Result<(), RuntimeError> {
        let component =
            wasmtime::component::Component::new(&self.engine, wasm).map_err(RuntimeError::Load)?;

        let mut linker = wasmtime::component::Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker).map_err(RuntimeError::Instantiate)?;

        let limits = self.build_limits(config);
        let ctx = self.build_wasi_ctx_p2(config);
        let state = P2State {
            ctx,
            table: wasmtime::component::ResourceTable::new(),
            limits,
        };
        let mut store = Store::new(&self.engine, state);

        store.limiter(|state| &mut state.limits);
        self.configure_epoch(&mut store, config);

        let _ticker = self.spawn_epoch_ticker(config);

        let command =
            wasmtime_wasi::bindings::sync::Command::instantiate(&mut store, &component, &linker)
                .map_err(RuntimeError::Instantiate)?;

        let result = command.wasi_cli_run().call_run(&mut store);

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(())) => Err(RuntimeError::Execute(anyhow::anyhow!(
                "component returned error"
            ))),
            Err(e) => self.handle_result(Err(e), config),
        }
    }

    // --- shared helpers ---

    fn build_limits(&self, config: &RuntimeConfig) -> StoreLimits {
        StoreLimitsBuilder::new()
            .memory_size(config.sandbox.max_memory)
            .table_elements(config.sandbox.max_table_elements)
            .instances(config.sandbox.max_instances)
            .build()
    }

    fn configure_epoch<T>(&self, store: &mut Store<T>, config: &RuntimeConfig) {
        let deadline_ticks = config.sandbox.max_time.as_secs().max(1);
        store.set_epoch_deadline(deadline_ticks);
        store.epoch_deadline_trap();
    }

    fn spawn_epoch_ticker(&self, config: &RuntimeConfig) -> std::thread::JoinHandle<()> {
        let epoch_engine = Arc::clone(&self.engine);
        let tick_interval = Duration::from_secs(1);
        let max_time = config.sandbox.max_time;
        std::thread::spawn(move || {
            let start = std::time::Instant::now();
            loop {
                std::thread::sleep(tick_interval);
                epoch_engine.increment_epoch();
                if start.elapsed() > max_time + tick_interval {
                    break;
                }
            }
        })
    }

    fn handle_result(
        &self,
        result: Result<(), anyhow::Error>,
        config: &RuntimeConfig,
    ) -> Result<(), RuntimeError> {
        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                let msg = format!("{e:?}");
                if msg.contains("epoch") || msg.contains("deadline") || msg.contains("interrupt") {
                    Err(RuntimeError::Timeout(config.sandbox.max_time))
                } else {
                    Err(RuntimeError::Execute(e))
                }
            }
        }
    }

    fn configure_wasi_builder(&self, builder: &mut WasiCtxBuilder, config: &RuntimeConfig) {
        if config.inherit_stdio {
            builder.inherit_stdio();
        }

        if !config.args.is_empty() {
            builder.args(&config.args);
        }

        for (key, value) in &config.env {
            builder.env(key, value);
        }

        // Mount directories from CLI --dir flags
        for mount in &config.fs.preopens {
            let dir_perms = if mount.writable {
                DirPerms::all()
            } else {
                DirPerms::READ
            };
            let file_perms = if mount.writable {
                FilePerms::all()
            } else {
                FilePerms::READ
            };
            let _ =
                builder.preopened_dir(&mount.host_path, &mount.guest_path, dir_perms, file_perms);
        }

        // Mount directories from sandbox policy
        for mount in &config.sandbox.fs {
            let dir_perms = if mount.permission == FsPermission::Rw {
                DirPerms::all()
            } else {
                DirPerms::READ
            };
            let file_perms = if mount.permission == FsPermission::Rw {
                FilePerms::all()
            } else {
                FilePerms::READ
            };
            let _ = builder.preopened_dir(&mount.host, &mount.guest, dir_perms, file_perms);
        }

        // Networking — deny by default, grant only if sandbox policy allows
        if config.sandbox.allow_tcp || config.sandbox.allow_udp {
            builder.inherit_network();
        }
        builder.allow_tcp(config.sandbox.allow_tcp);
        builder.allow_udp(config.sandbox.allow_udp);
        builder.allow_ip_name_lookup(config.sandbox.allow_dns);
    }

    fn build_wasi_ctx_p1(&self, config: &RuntimeConfig) -> WasiP1Ctx {
        let mut builder = WasiCtxBuilder::new();
        self.configure_wasi_builder(&mut builder, config);
        builder.build_p1()
    }

    fn build_wasi_ctx_p2(&self, config: &RuntimeConfig) -> WasiCtx {
        let mut builder = WasiCtxBuilder::new();
        self.configure_wasi_builder(&mut builder, config);
        builder.build()
    }
}

/// Detect whether a WASM binary is a component (P2) or a core module (P1).
/// Components start with the component magic bytes: `\0asm` followed by version + layer byte.
fn is_component(wasm: &[u8]) -> bool {
    // Core module: \0asm\x01\x00\x00\x00
    // Component:   \0asm\x0d\x00\x01\x00
    wasm.len() >= 8 && wasm[0..4] == *b"\0asm" && wasm[4] == 0x0d
}
