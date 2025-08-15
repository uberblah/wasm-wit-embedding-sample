mod pollable;

use wasmtime::Config;
use wasmtime::Engine;
use wasmtime::Store;
use wasmtime::component::Component;
use wasmtime::component::Linker;
use wasmtime::component::TypedFunc;
use wasmtime::component::bindgen;

use crate::pollable::{Pollable, PollableResult};

bindgen!({async: true});

#[derive(Debug)]
pub enum SbHarnessError {
    EngineInitialization(wasmtime::Error),
    BinaryLoading(wasmtime::Error),
    LinkerSetup(wasmtime::Error),
    ProgramInstantiation(wasmtime::Error),
    ProcessRefueling(wasmtime::Error),
    ProcessRun(wasmtime::Error),
}

impl std::fmt::Display for SbHarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SbHarnessError::EngineInitialization(e) => {
                write!(f, "Failed to initialize engine: {}", e)
            }
            SbHarnessError::BinaryLoading(e) => write!(f, "Failed to create component: {}", e),
            SbHarnessError::LinkerSetup(e) => write!(f, "Failed to set up linker: {}", e),
            SbHarnessError::ProgramInstantiation(e) => {
                write!(f, "Failed to instantiate program: {}", e)
            }
            SbHarnessError::ProcessRefueling(e) => write!(f, "Failed to refuel process: {}", e),
            SbHarnessError::ProcessRun(e) => write!(f, "Failed to run process: {}", e),
        }
    }
}

impl std::error::Error for SbHarnessError {}

pub struct ShellboundState {
    pub process_id: u32,
    pub process_name: String,
    pub stdin_waiting: Vec<u8>,
}

#[async_trait::async_trait]
impl ProgramImports for ShellboundState {
    async fn write_stdout(&mut self, data: Vec<u8>) -> Result<i32, wasmtime::Error> {
        println!(
            "[write_stdout][t={}][pid={}]: {}",
            chrono::Local::now().format("%H:%M:%S%.3f"),
            self.process_id,
            self.process_name,
        );
        Ok(data.len() as i32)
    }

    async fn read_stdin(&mut self, max_bytes: u32) -> Result<Vec<u8>, wasmtime::Error> {
        let n_to_read = self.stdin_waiting.len().min(max_bytes as usize);
        Ok(self.stdin_waiting.drain(..n_to_read).collect())
    }
}

pub struct SbHarnessFactory {
    engine: Engine,
    linker: Linker<ShellboundState>,
}

impl SbHarnessFactory {
    pub fn new() -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.consume_fuel(true);
        config.async_support(true);
        let engine =
            wasmtime::Engine::new(&config).map_err(SbHarnessError::EngineInitialization)?;
        let mut linker = Linker::new(&engine);
        Program::add_to_linker(&mut linker, |state: &mut ShellboundState| state)
            .map_err(SbHarnessError::LinkerSetup)?;
        Ok(SbHarnessFactory { engine, linker })
    }

    pub fn create(
        &self,
        wasm_bytes: &[u8],
        process_id: u32,
        process_name: String,
        args: String,
        fuel_per_step: u64,
    ) -> Result<SbHarness, SbHarnessError> {
        let component = Component::from_binary(&self.engine, &wasm_bytes)
            .map_err(SbHarnessError::BinaryLoading)?;
        let mut store = Store::new(
            &self.engine,
            ShellboundState {
                process_id,
                process_name,
                stdin_waiting: args.into_bytes(),
            },
        );
        store
            .fuel_async_yield_interval(Some(fuel_per_step))
            .map_err(SbHarnessError::ProcessRefueling)?;
        let (_bindings, instance) = futures::executor::block_on(Program::instantiate_async(
            &mut store,
            &component,
            &self.linker,
        ))
        .map_err(SbHarnessError::ProgramInstantiation)?;

        store
            .set_fuel(u64::MAX)
            .map_err(SbHarnessError::ProcessRefueling)?;

        let process_run_func: TypedFunc<(), (u32,)> = instance
            .exports(&mut store)
            .root()
            .typed_func("process-run")
            .map_err(SbHarnessError::ProcessRun)?;

        let fut = async move { process_run_func.call_async(&mut store, ()).await };

        let pollable = Pollable::new(Box::new(fut));

        Ok(SbHarness { pollable })
    }
}

pub struct SbHarness {
    pollable: Pollable<Result<(u32,), wasmtime::Error>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RunResult {
    Complete(u32),
    Incomplete,
}

impl SbHarness {
    pub fn step(&mut self) -> Result<RunResult, SbHarnessError> {
        let poll_result = self.pollable.poll();
        match poll_result {
            PollableResult::Ready(Ok((result,))) => Ok(RunResult::Complete(result)),
            PollableResult::Ready(Err(e)) => {
                eprintln!("Error during process run: {}", e);
                Err(SbHarnessError::ProcessRun(e))
            }
            PollableResult::Pending => Ok(RunResult::Incomplete),
            PollableResult::Stale => {
                eprintln!("Process has already completed, cannot step again.");
                Err(SbHarnessError::ProcessRun(wasmtime::Error::msg(
                    "Process has already completed",
                )))
            }
        }
    }
}
