use crate::system::SystemId;
use crate::{System, hollywood::ipc::IosDevice};
use std::collections::{HashMap, VecDeque};

// As per zayd
const FINALIZE_DELAY_CYCLES: u64 = 10_000;
const ACK_TO_NEXT_DELAY_CYCLES: u64 = 500;

pub struct PendingResponse {
    pub cmd_paddr: u32,
    pub result: i32,
}

pub struct Starlet {
    devices: HashMap<String, Box<dyn IosDevice>>,
    handles: HashMap<i32, String>,
    next_fd: i32,
    pub pending: VecDeque<PendingResponse>,
}

impl Starlet {
    pub fn new() -> Self {
        Starlet {
            devices: HashMap::new(),
            handles: HashMap::new(),
            next_fd: 1,
            pending: VecDeque::new(),
        }
    }

    /// Install a device at `path`. Replaces any existing registration.
    pub fn register(&mut self, path: &str, dev: Box<dyn IosDevice>) {
        self.devices.insert(path.to_owned(), dev);
    }

    /// Allocate a fresh fd and bind it to `path`.
    pub fn allocate_fd(&mut self, path: &str) -> i32 {
        let fd = self.next_fd;
        self.next_fd = self.next_fd.checked_add(1).expect("Starlet fd overflow");
        self.handles.insert(fd, path.to_owned());
        fd
    }

    /// Drop the fd to path mapping. Returns the path the fd was bound to.
    pub fn release_fd(&mut self, fd: i32) -> Option<String> {
        self.handles.remove(&fd)
    }

    pub fn device_for_path(&mut self, path: &str) -> Option<&mut Box<dyn IosDevice>> {
        self.devices.get_mut(path)
    }

    pub fn device_for_fd(&mut self, fd: i32) -> Option<&mut Box<dyn IosDevice>> {
        let path = self.handles.get(&fd)?.clone();
        self.devices.get_mut(&path)
    }
}

impl System<{ crate::WII }> {
    pub fn initialize_starlet_devices(&mut self) {
        self.starlet
            .register("/dev/stm/immediate", Box::new(crate::hollywood::ipc::stm::Stm));
    }
}

pub fn dispatch_command<const SYSTEM: SystemId>(sys: &mut System<SYSTEM>, cmd_paddr: u32) {
    let result = self::process_command(sys, cmd_paddr);

    sys.starlet.pending.push_back(PendingResponse { cmd_paddr, result });
    sys.scheduler
        .schedule_in(FINALIZE_DELAY_CYCLES, self::deliver_pending::<SYSTEM>);
}

pub fn schedule_drain<const SYSTEM: SystemId>(sys: &mut System<SYSTEM>) {
    sys.scheduler
        .schedule_in(ACK_TO_NEXT_DELAY_CYCLES, self::deliver_pending::<SYSTEM>);
}

fn process_command<const SYSTEM: SystemId>(sys: &mut System<SYSTEM>, cmd_paddr: u32) -> i32 {
    let cmd = sys.mmio.phys_read_u32(cmd_paddr);
    tracing::warn!(
        cmd,
        paddr = format!("{cmd_paddr:#010X}"),
        "Starlet command decoder not implemented; returning IPC_ENOENT"
    );
    
    crate::hollywood::ipc::IPC_ENOENT
}

fn deliver_pending<const SYSTEM: SystemId>(sys: &mut System<SYSTEM>) {
    if sys.hollywood.ipc.ppcctrl.ack_reply() {
        // PPC slot still occupied. Wait for the ack.
        return;
    }

    let Some(p) = sys.starlet.pending.pop_front() else {
        return;
    };
    crate::hollywood::ipc::deliver_response(sys, p.cmd_paddr, p.result);
}
