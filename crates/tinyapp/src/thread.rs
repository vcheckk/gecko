use gecko::HostInput;
use gecko::system::{System, SystemId};
use spin_sleep::SpinSleeper;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use winit::event_loop::EventLoopProxy;

pub fn emu_thread<const SYSTEM: SystemId>(
    mut emulator: System<SYSTEM>,
    input: Arc<Mutex<HostInput>>,
    proxy: EventLoopProxy<()>,
    game_id: Option<String>,
    throttle: bool,
    start_gate: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
) {
    let sleeper = SpinSleeper::default();
    let throttle_step = Duration::from_micros(500);

    while !start_gate.load(Ordering::Acquire) {
        if shutdown.load(Ordering::Relaxed) {
            return;
        }
        sleeper.sleep(Duration::from_millis(10));
    }

    loop {
        while throttle && emulator.audio_sink.should_throttle() {
            sleeper.sleep(throttle_step);
        }

        let input = *input.lock().unwrap();
        emulator.apply_host_input(&input);
        emulator.run_until_vsync();

        if proxy.send_event(()).is_err() {
            break;
        }
    }

    if let Some(game_id) = game_id.as_deref() {
        match emulator.save_jit_cache(game_id) {
            Ok((ppc, dsp, vtx)) => {
                tracing::info!(ppc_blocks = ppc, dsp_blocks = dsp, vtx_keys = vtx, "saved JIT cache")
            }
            Err(err) => tracing::warn!(?err, "failed to save JIT cache"),
        }
    }
}
