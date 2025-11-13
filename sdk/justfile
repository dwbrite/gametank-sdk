ensure-container-running:
    podman ps --filter "name=gametank" --filter "status=running" --format "{{{{.Names}}}}" | grep -q gametank || \
    podman run -d --name gametank -v $(pwd):/workspace:z --replace rust-mos:gte sleep infinity

# Compile all .asm files into .o files
assemble-asm-files: ensure-container-running
    podman exec -t -w /workspace/rom gametank find . -name "*.asm" -exec bash -c ' \
        filename=$(basename "{}" .asm); \
        echo "Assembling $filename..."; \
        llvm-mc --filetype=obj -triple=mos -mcpu=mosw65c02 "{}" -o "target/asm/$filename.o"' \;

# Archive the .o files into libasm.a
archive-asm-files: ensure-container-running
    podman exec -t -w /workspace/rom gametank bash -c ' \
        llvm-ar rcs target/asm/libasm.a target/asm/*.o && \
        rm target/asm/*.o'

# Full build-asm task
build-asm: assemble-asm-files archive-asm-files

# Objcopy a compiled rom to output.bin
objcopy: ensure-container-running
    podman exec -t -w /workspace/rom gametank llvm-objcopy -O binary target/mos-unknown-none/release/rom rom.gtr

# Objdump a compiled rom
objdump: ensure-container-running
    podman exec -t -w /workspace/rom gametank llvm-objdump -d --triple=mos target/mos-unknown-none/release/rom

# Build rom, compiling asm first
build: build-asm
    podman exec -t -w /workspace/rom gametank cargo +mos build --release -Z build-std=core --target mos-unknown-none
    # just objcopy

# Run cargo fix with --allow-dirty
fix:
    podman exec -t -w /workspace/rom gametank cargo +mos fix --allow-dirty --release -Z build-std=core --target mos-unknown-none

