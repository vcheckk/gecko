use std::fs::File;
use std::io::BufWriter;

use dbglib::Debugger;
use image::symbols::SymbolTable;

const TRACE_FILENAME: &str = "trace.log";

pub struct DebuggerUi {
    pub debugger: Debugger,
    pub symbols: Option<SymbolTable>,
    pub show_cpu: bool,
    pub show_dsp: bool,
    pub show_gx_state: bool,
    pub show_mmio: bool,
    pub show_dvd: bool,
    pub show_exi: bool,
    pub show_irqs: bool,
    pub show_controls: bool,
    pub show_callstack: bool,
    pub memory_base: u32,
    pub memory_addr_input: String,
    pub run_until_addr_input: String,
    pub dvd_cover_open: Option<bool>,
}

impl Default for DebuggerUi {
    fn default() -> Self {
        DebuggerUi {
            debugger: Debugger::new(),
            symbols: None,
            show_cpu: true,
            show_dsp: false,
            show_controls: true,
            show_gx_state: false,
            show_mmio: false,
            show_dvd: false,
            show_exi: false,
            show_irqs: false,
            show_callstack: false,
            memory_base: 0x8000_0000,
            memory_addr_input: "80000000".to_string(),
            run_until_addr_input: String::new(),
            dvd_cover_open: None,
        }
    }
}

impl DebuggerUi {
    pub fn start_trace(&mut self) {
        let file = File::create(TRACE_FILENAME).expect("failed to create trace file");
        self.debugger.start_trace(Box::new(BufWriter::new(file)));
    }

    pub fn stop_trace(&mut self) {
        self.debugger.stop_trace();
    }
}
