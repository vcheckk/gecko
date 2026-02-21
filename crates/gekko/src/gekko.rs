use crate::{cpu, mmu, scheduler};

pub struct Gekko {
    pub cpu: cpu::Cpu,
    pub scheduler: scheduler::Scheduler,
    pub mmu: mmu::Mmu,
}

impl Gekko {
    pub fn new(path: &str) -> Self {
        let mut mmu = mmu::Mmu::new();
        let data = std::fs::read(path).expect("failed to read ROM");
        mmu.memory[..data.len()].copy_from_slice(&data);

        Gekko {
            cpu: cpu::Cpu::new(),
            scheduler: scheduler::Scheduler { cycles: 0 },
            mmu,
        }
    }

    pub fn run_until_event(&mut self) {
        self.cpu.cia = self.cpu.pc;
        self.cpu.nia = self.cpu.cia.wrapping_add(4);

        let instr = cpu::semantics::Instruction(self.mmu.read_u32(self.cpu.cia));
        cpu::lut::dispatch(self, instr);
        self.scheduler.cycles += 1;
        
        self.cpu.pc = self.cpu.nia;
    }
}
