use gecko::HostInput;
use gecko::system::{System, SystemId};
use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoopProxy;

pub fn emu_thread<const SYSTEM: SystemId>(
    mut emulator: System<SYSTEM>,
    input: Arc<Mutex<HostInput>>,
    proxy: EventLoopProxy<()>,
) {
    loop {
        let input = *input.lock().unwrap();
        emulator.apply_host_input(&input);
        emulator.run_until_vsync();

        if proxy.send_event(()).is_err() {
            break;
        }
    }
}
