use crate::blockchain::state::BlockchainState;
use crate::crypto::hash::Hash;
use crate::runtime::program::ProgramId;
use anyhow::{Result, anyhow};
use solana_rbpf::{
    aligned_memory::AlignedMemory,
    assembler::assemble,
    ebpf,
    elf::Executable,
    memory_region::{MemoryMapping, MemoryRegion},
    program::{BuiltinProgram, FunctionRegistry},
    static_analysis::Analysis,
    verifier::RequisiteVerifier,
    vm::{Config, DynamicAnalysis, EbpfVm, TestContextObject},
};
use std::sync::{Arc, RwLock};

/// Results of a program execution
#[derive(Debug)]
pub struct ExecutionResult {
    /// Return value from the program
    pub return_value: u64,

    /// Gas used during execution
    pub gas_used: u64,
    // /// Logs produced during execution
    // pub logs: Vec<String>,
}

/// eBPF Virtual Machine for executing programs
pub struct VirtualMachine {
    /// Current blockchain state
    state: Arc<RwLock<BlockchainState>>,

    /// VM configuration
    config: Config,

    /// Gas limit for execution
    gas_limit: u64,
}

impl VirtualMachine {
    /// Create a new virtual machine
    pub fn new(state: Arc<RwLock<BlockchainState>>, gas_limit: u64) -> Self {
        // Configure the VM
        let config = Config {
            max_call_depth: 10,
            stack_frame_size: 4096,
            enable_instruction_meter: true,
            enable_instruction_tracing: false,
            ..Config::default()
        };

        Self {
            state,
            config,
            gas_limit,
        }
    }

    /// Execute a program
    pub fn execute(&self, program_id: &ProgramId, input_data: &[u8]) -> Result<ExecutionResult> {
        // Get state for reading
        let state = self.state.read().unwrap();
        let loader = Arc::new(BuiltinProgram::new_loader(
            Config {
                enable_instruction_tracing: false,
                enable_symbol_and_section_labels: true,
                ..Config::default()
            },
            FunctionRegistry::default(),
        ));

        let program_info = state
            .programs
            .get(program_id)
            .ok_or_else(|| anyhow!("Program not found: {}", program_id))
            .unwrap();

        let mut executable = Executable::from_elf(&program_info.code, loader)
            .map_err(|e| anyhow!("Failed to load ELF: {}", e))
            .unwrap();

        executable.verify::<RequisiteVerifier>().unwrap();

        let mut mem = vec![0u8; input_data.len()];

        #[cfg(all(not(target_os = "windows"), target_arch = "x86_64"))]
        executable.jit_compile().unwrap();

        let mut context_object = TestContextObject::new(u64::MAX);

        let config = executable.get_config();
        let sbpf_version = executable.get_sbpf_version();

        let mut stack = AlignedMemory::<{ ebpf::HOST_ALIGN }>::zero_filled(config.stack_size());
        let stack_len = stack.len();

        let mut heap = AlignedMemory::<{ ebpf::HOST_ALIGN }>::zero_filled(4096);

        let regions: Vec<MemoryRegion> = vec![
            executable.get_ro_region(),
            MemoryRegion::new_writable_gapped(
                stack.as_slice_mut(),
                ebpf::MM_STACK_START,
                if !sbpf_version.dynamic_stack_frames() && config.enable_stack_frame_gaps {
                    config.stack_frame_size as u64
                } else {
                    0
                },
            ),
            MemoryRegion::new_writable(heap.as_slice_mut(), ebpf::MM_HEAP_START),
            MemoryRegion::new_writable(&mut mem, ebpf::MM_INPUT_START),
        ];

        let memory_mapping = MemoryMapping::new(regions, config, sbpf_version).unwrap();

        let mut vm = EbpfVm::new(
            executable.get_loader().clone(),
            executable.get_sbpf_version(),
            &mut context_object,
            memory_mapping,
            stack_len,
        );

        let (instruction_count, result) = vm.execute_program(&executable, false);
        println!("Result: {result:?}");
        println!("Instruction Count: {instruction_count}");

        let return_value = result.unwrap();

        Ok(ExecutionResult {
            return_value,
            gas_used: instruction_count,
        })
    }
}

/// Symbolic execution context for testing programs
pub struct TestContext {
    /// Virtual machine for execution
    vm: VirtualMachine,

    /// Mock blockchain state
    state: Arc<RwLock<BlockchainState>>,
}

impl TestContext {
    /// Create a new test context
    pub fn new() -> Self {
        let state = Arc::new(RwLock::new(BlockchainState::new()));
        let vm = VirtualMachine::new(Arc::clone(&state), 1_000_000);

        Self { vm, state }
    }

    /// Run a program in the test context
    pub fn run_program(&self, program_code: &[u8], input_data: &[u8]) -> Result<ExecutionResult> {
        // Hash the program code to get ID
        let program_id = Hash::hash(program_code);

        // Register the program in the state
        {
            let mut state = self.state.write().unwrap();
            state.programs.insert(
                program_id,
                crate::blockchain::state::ProgramInfo {
                    code: program_code.to_vec(),
                    owner: crate::crypto::keys::PublicKey::from_bytes(&[0; 32])?,
                    deployment_slot: 0,
                },
            );
        }

        // Execute
        self.vm.execute(&program_id, input_data)
    }
}
