use crate::system::{System, SystemId};

#[inline(always)]
pub fn ipc_read<const SYSTEM: SystemId>(_sys: &mut System<SYSTEM>, _addr: u32, _size: u32) -> Option<u32> {
    Some(0)
}

#[inline(always)]
pub fn ipc_write<const SYSTEM: SystemId>(_sys: &mut System<SYSTEM>, _addr: u32, _size: u32, _val: u32) -> bool {
    true
}
