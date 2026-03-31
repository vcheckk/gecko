use disasm::gekko::GekkoInstruction;
use disasm::tokenizer::{self, AsmToken};
use gecko::gamecube::GameCube;

const DISASM_COL: usize = 22;
const COMMENT_COL: usize = 50;

/// Format a single trace line for the current PC of the emulator.
///
/// Returns a string like `80003100  4E800020  blr                           ; lr=80001234`
pub fn format_trace_line(emulator: &GameCube) -> String {
    let pc = emulator.cpu.pc;
    let raw = emulator.mmio.virt_read_u32(pc);

    if let Some((instr, _)) = GekkoInstruction::decode(emulator.mmio.virt_slice(pc, 4)) {
        let text = format!("{}", instr);
        let comment = reg_comment(&text, &emulator.cpu.gprs, &emulator.cpu.fprs);
        let pad = COMMENT_COL.saturating_sub(DISASM_COL + text.len());
        if comment.is_empty() {
            format!("{:08X}  {:08X}  {}", pc, raw, text)
        } else {
            format!("{:08X}  {:08X}  {}{}; {}", pc, raw, text, " ".repeat(pad), comment)
        }
    } else {
        format!("{:08X}  {:08X}  <unknown>", pc, raw)
    }
}

/// Generate register-value comments for a disassembled instruction line.
///
/// Parses the disassembly text for register references and returns a comma-separated
/// list of their current values, e.g. `r3=00000001, r4=80003000`.
pub fn reg_comment(disasm_text: &str, gprs: &[u32; 32], fprs: &[f64; 32]) -> String {
    let tokens = tokenizer::tokenize(disasm_text);
    let mut parts = Vec::new();
    let mut gpr_seen = [false; 32];
    let mut fpr_seen = [false; 32];
    for tok in &tokens {
        match tok {
            AsmToken::Gpr(n) => {
                let n = *n as usize;
                if !gpr_seen[n] {
                    gpr_seen[n] = true;
                    parts.push(format!("r{}={:08X}", n, gprs[n]));
                }
            }
            AsmToken::Fpr(n) => {
                let n = *n as usize;
                if !fpr_seen[n] {
                    fpr_seen[n] = true;
                    parts.push(format!("f{}={:.6e}", n, fprs[n]));
                }
            }
            _ => {}
        }
    }
    parts.join(", ")
}
