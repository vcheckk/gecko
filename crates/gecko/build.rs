fn main() {
    chipi_build::generate_bindings("gekko.bindings.chipi").expect("chipi codegen failed (gekko)");
    chipi_build::generate_bindings("wii.bindings.chipi").expect("chipi codegen failed (wii)");
    chipi_build::generate_bindings("dsp.bindings.chipi").expect("chipi codegen failed (dsp)");
}
