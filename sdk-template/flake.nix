# GameTank ROM development environment
#
# Provides rustup and the mos-llvm Rust toolchain needed to build ROMs
# targeting the GameTank's 65C02 CPU, mirroring the docker.io/dwbrite/rust-mos:gte
# container used by `gtrom build`. `shellHook` links a prebuilt rust-mos
# compiler as the mos rustup toolchain and puts mos-clang/lld
# (from llvm-mos-sdk, used by rustc as the linker for this target)
# and ca65/ld65 (used to assemble the FM audio firmware) on $PATH.

#
# Usage:
#   nix develop  # enter dev shell; installs rustup and links rust-mos
#   cd <your-game-directory>
#   cargo +mos build --release
#   gtrom convert target/mos-unknown-none/release/rom -o rom.gtr
#   gte rom.gtr
#
{
  description = "GameTank ROM development flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        # github:mrk-its/rust-mos @ rust-mos-ubuntu-24.04
        # autoPatchelfHook rewrites ELF interpreter + RPATHs for NixOS
        rust-mos = pkgs.stdenv.mkDerivation {
          name = "rust-mos-1.87.0-dev";

          # Two tarballs installed into the same prefix:
          #   1. rust-1.87.0-dev - compiler, cargo, std libs (arch-specific)
          #   2. rust-src-1.87.0-dev - library source required by -Z build-std
          srcs = [
            (pkgs.fetchurl {
              url = "https://github.com/mrk-its/rust-mos/releases/download/rust-mos-ubuntu-24.04/rust-1.87.0-dev-x86_64-unknown-linux-gnu.tar.gz";
              hash = "sha256-csyzyF6fwdZZlFRY/M3uYIICshgb/1o3SHda4rahbTY=";
            })
            (pkgs.fetchurl {
              url = "https://github.com/mrk-its/rust-mos/releases/download/rust-mos-ubuntu-24.04/rust-src-1.87.0-dev.tar.gz";
              hash = "sha256-ITPFUXmjEkrg2361Izjt1siz/ZoaZkfBe+81fT1ZuvY=";
            })
          ];

          nativeBuildInputs = with pkgs; [
            autoPatchelfHook
          ];

          # Libraries required by rustc / cargo binaries at runtime
          buildInputs = with pkgs; [
            zlib
            openssl # libssl.so.3 + libcrypto.so.3 (cargo)
            stdenv.cc.cc.lib # libstdc++.so.6, libgcc_s.so.1
          ];

          dontConfigure = true;
          dontBuild = true;

          unpackPhase = ''
            for src in $srcs; do
              tar xzf "$src"
            done
          '';

          installPhase = ''
            runHook preInstall
            bash rust-1.87.0-dev-x86_64-unknown-linux-gnu/install.sh \
              --prefix="$out" \
              --disable-ldconfig \
              --without=rust-docs
            bash rust-src-1.87.0-dev/install.sh \
              --prefix="$out" \
              --disable-ldconfig
            runHook postInstall
          '';
        };

        # github:llvm-mos/llvm-mos-sdk release v23.0.1 - provides mos-clang/lld/llvm-ar
        llvm-mos = pkgs.stdenv.mkDerivation {
          name = "llvm-mos-sdk-23.0.1";

          src = pkgs.fetchurl {
            url = "https://github.com/llvm-mos/llvm-mos-sdk/releases/download/v23.0.1/llvm-mos-linux.tar.xz";
            hash = "sha256-vXbdWoJrg+Q4v1Xb3VEtZdRgC6gP2pGdymM8rXbwI8w=";
          };

          nativeBuildInputs = with pkgs; [
            autoPatchelfHook
          ];

          buildInputs = with pkgs; [
            zlib
            stdenv.cc.cc.lib # libstdc++.so.6, libgcc_s.so.1
          ];

          dontConfigure = true;
          dontBuild = true;

          installPhase = ''
            runHook preInstall
            mkdir -p "$out"
            cp -r . "$out"/
            # The release tarball ships mos-clang/mos-clang++ as symlinks to
            # a plain "clang"/"clang++" name that isn't actually included -
            # only the versioned clang-23 binary is. Recreate the missing
            # link so mos-clang -> clang -> clang-23 resolves.
            ln -sf clang-23 "$out"/bin/clang
            runHook postInstall
          '';
        };

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs =
            with pkgs;
            [
              cc65 # assembles the FM audio coprocessor firmware
              pkg-config
              rustup
            ]
            ++ pkgs.lib.optionals (pkgs.lib.strings.hasInfix "linux" system) [
              alsa-lib
              libclang # needed to recompile the sdk crate with libretro-rs-ffi
              libudev-zero
              libx11
              libxi
              libxcursor
              SDL2 # needed to run C-based emulator
              vulkan-loader
            ]
            ++ [ llvm-mos ]; # provides mos-clang/lld used by build scripts and rustc

          shellHook = ''
            # Expose cargo (managed by rustup) and the llvm-mos toolchain
            # (mos-clang/lld) that rustc invokes to link.
            export PATH="$HOME/.cargo/bin:$PATH"

            # Link rust-mos as the 'mos' named toolchain so that
            #   cargo +mos build -Z build-std=core --target mos-unknown-none
            # works without a container.
            rustup toolchain link mos ${rust-mos} 2>/dev/null \
              && echo "rust-mos: linked as 'mos' toolchain (${rust-mos})" \
              || echo "rust-mos: 'mos' toolchain already linked"

            echo ""
            echo "Build ROM with the commands:"
            echo "cargo +mos build --release"
            echo "gtrom convert target/mos-unknown-none/release/rom -o rom.gtr"
            echo ""
            echo "And launch the ROM via the emulator:"
            echo "gte rom.gtr"
          '';

          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.glibc.dev}/include";

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
            with pkgs;
            [
              libx11
              libxi
              libxcursor
              libxkbcommon
              vulkan-loader
            ]
          );
        };
      }
    );
}
