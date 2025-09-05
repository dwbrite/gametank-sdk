mkdir rom/target
mkdir rom/target/asm

➜ podman run -d --name gametank -v $(pwd):/workspace:z --replace rust-mos:gte sleep infinity
➜ podman exec -it -w /workspace gametank /bin/zsh

➜ find . -name "*.asm" -exec bash -c 'filename=$(basename "{}" .asm); echo "Assembling $filename..."; llvm-mc --filetype=obj -triple=mos -mcpu=mosw65c02 "{}" -o "target/asm/$filename.o"' \;


➜  bash -c 'llvm-ar rcs target/asm/libasm.a target/asm/*.o && rm target/asm/*.o'

➜ llvm-objdump -s target/mos-unknown-none/release/main

➜ cargo +mos build --release -Z build-std=core --target mos-unknown-none


---

IMPORTANT

After just/cargo build-ing, use gtrom to pack elf into a raw binary
