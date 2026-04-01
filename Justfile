pgo_dir := "/tmp/gecko-pgo-data"
base_flags := "-Ctarget-cpu=native -Ccodegen-units=1"

pgo package +training_args:
    @echo "=== PGO build for {{package}} ==="
    @echo "[1/4] Building instrumented binary..."
    rm -rf {{pgo_dir}}
    mkdir -p {{pgo_dir}}
    RUSTFLAGS="{{base_flags}} -Cprofile-generate={{pgo_dir}}" cargo build -p {{package}} --release
    @echo "[2/4] Training run..."
    ./target/release/{{package}} {{training_args}}
    @echo "[3/4] Merging profile data..."
    llvm-profdata merge -o {{pgo_dir}}/merged.profdata {{pgo_dir}}
    @echo "[4/4] Building PGO-optimized binary..."
    RUSTFLAGS="{{base_flags}} -Cprofile-use={{pgo_dir}}/merged.profdata" cargo build -p {{package}} --release
    @echo ""
    @echo "Done. Binary at: target/release/{{package}}"

pgo-tinybench frames="500":
    just pgo tinybench --ipl private/IPL.decoded.bin --dsp private/dsp_rom.bin --frames {{frames}}

pgo-tinyapp:
    just pgo tinyapp --ipl private/IPL.decoded.bin --dsp private/dsp_rom.bin
