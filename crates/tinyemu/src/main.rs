use colored::Colorize;
use disasm::gekko::GekkoInstruction;
use disasm::tokenizer::{self, AsmToken};

#[derive(Clone, Copy)]
struct CpuSnapshot {
    gprs: [u32; 32],
    lr: u32,
    ctr: u32,
}

impl CpuSnapshot {
    fn from_cpu(cpu: &gekko::cpu::Cpu) -> Self {
        Self {
            gprs: cpu.gprs,
            lr: cpu.lr,
            ctr: cpu.ctr,
        }
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Usage: debugger <path_to_rom>");
    let is_debug = std::env::args().any(|arg| arg == "--debug");

    let mut gekko = gekko::gekko::Gekko::new(&path);
    let mut prev_snapshot = CpuSnapshot::from_cpu(&gekko.cpu);
    
    loop {
        let addr = gekko.cpu.pc;
        let instr = GekkoInstruction::decode(&gekko.mmu.memory[addr as usize..])
            .expect("failed to decode instruction")
            .0;

        if is_debug {
            dbg!(&instr);
        }

        println!(
            "{}: {}",
            format!("{:08X}", addr).bold(),
            colorize_instr(&instr)
        );

        gekko.run_until_event();
        let curr_snapshot = CpuSnapshot::from_cpu(&gekko.cpu);

        if is_debug {
            dump_registers(&curr_snapshot, &prev_snapshot);
        }

        prev_snapshot = curr_snapshot;
    }
}

fn colorize(tok: &AsmToken<'_>) -> String {
    match tok {
        AsmToken::Mnemonic(s) => s.bold().cyan().to_string(),
        AsmToken::Gpr(n) => format!("r{n}").yellow().to_string(),
        AsmToken::Fpr(n) => format!("f{n}").magenta().to_string(),
        AsmToken::CrField(n) => format!("cr{n}").green().to_string(),
        AsmToken::Spr(s) => s.green().bold().to_string(),
        AsmToken::ImmSigned(v) => format!("{v}").blue().to_string(),
        AsmToken::ImmUnsigned(v) => format!("{v}").blue().to_string(),
        AsmToken::ImmHex(v) if *v < 0 => format!("-0x{:X}", -v).blue().to_string(),
        AsmToken::ImmHex(v) => format!("0x{v:X}").blue().to_string(),
        AsmToken::Displacement(v) => format!("{v}").blue().to_string(),
        AsmToken::BranchTarget(s) => s.bright_red().to_string(),
        AsmToken::Punct(_) | AsmToken::Text(_) => tok.to_string(),
    }
}

fn dump_registers(curr: &CpuSnapshot, prev: &CpuSnapshot) {
    let fmt_reg = |label: &str, val: u32, prev_val: u32| -> String {
        let value = format!("{:08X}", val);
        if val != prev_val {
            format!("{} {} ", label.yellow().bold(), value.bright_red().bold())
        } else {
            format!("{} {} ", label.dimmed(), value.dimmed())
        }
    };

    for row in 0..8 {
        let line: String = (0..4)
            .map(|col| {
                let i = row * 4 + col;
                fmt_reg(&format!("r{:<2}", i), curr.gprs[i], prev.gprs[i])
            })
            .collect();
        println!("{}", line.trim_end());
    }

    println!(
        "{}",
        format!(
            "{}{}",
            fmt_reg("lr ", curr.lr, prev.lr),
            fmt_reg("ctr", curr.ctr, prev.ctr)
        )
        .trim_end()
    );

    println!();
}

fn colorize_instr(instr: &disasm::gekko::GekkoInstruction) -> String {
    let text = format!("{}", instr);
    let tokens = tokenizer::tokenize(&text);
    tokens
        .into_iter()
        .map(|t| colorize(&t))
        .collect::<Vec<_>>()
        .join("")
}
