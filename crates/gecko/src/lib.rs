pub mod cpu;
pub mod dvd;
pub mod flipper;
pub mod gamecube;
pub mod idle;
pub mod mmio;
pub mod scheduler;

#[cfg(feature = "scripting")]
pub mod scripting;
