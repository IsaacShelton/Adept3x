# How to Compile

Unfortunately, since we rely on LLVM, compiling from scratch can be painful.

This guide is meant to reduce that pain. Different system configurations may
yield different results.

### Windows

Rust Setup:

- Download rustup
- Continue WITHOUT installing the C++ build tools
- Press `2` for customization
- Enter the default target triple as `x86_64-pc-windows-gnu`
- Enter nightly as default toolchain
- Choose default installation profile
- Tell it to modify the path variable
- Press `1` to proceed with the installation

MSYS2 Setup:

- Download MSYS2
- Install MSYS2
- Open the MSYS2 Mingw64 Prompt
- `pacman -S git --no-confirm`
- `pacman -S mingw-w64-x86_64-gcc --no-confirm`
- `pacman -S mingw-w64-x86_64-llvm --no-confirm`
- `git clone https://github.com/IsaacShelton/Adept3x`
- `cd Adept3x`
- `LLVM_SYS_181_PREFIX=/mingw64 ~/.cargo/bin/cargo +nightly build --release`

The output will be in `./target/release`.

Then, either copy the `./infrastructure` folder to
`./target/release/infrastructure`, or specify the infrastructure to use via the
command line - `./target/release/adept.exe --infrastructure ./infrastructure`.

This folder contains files for linking on Windows, as well as files needed for
cross-compilation.

### MacOS

- Install rust nightly (most likely via rustup)
- `brew install llvm-18-dev`
- `cargo +nightly build --release`
- (optional) copy `./infrastructure` folder to `./target/release/infrastructure`
  for easy cross-compiling to other platforms

### Linux

- Install rust nightly (most likely via rustup)
- `sudo apt install -y git`:
- `sudo apt install -y llvm-18-dev`:
- `sudo apt install -y libstdc++14-dev`:
- `git clone https://github.com/IsaacShelton/Adept3x`
- `cd Adept3x`
- `RUSTFLAGS='-L /usr/lib/gcc/x86_64-linux-gnu/14/' cargo +nightly build --release`
- (optional) copy `./infrastructure` folder to `./target/release/infrastructure`
  for easy cross-compiling to other platforms

### FreeBSD

- `pkg install git`
- `pkg install devel/llvm18`
- `pkg install lang/rust-nightly`
- `git clone https://github.com/IsaacShelton/Adept3x`
- `cd Adept3x`
- `cargo +nightly build --release`
- (optional) copy `./infrastructure` folder to `./target/release/infrastructure`
  for easy cross-compiling to other platforms

# FAQ

### Where do the binaries in `infrastructure/` come from?

These necessary files are packaged for convenience, but feel free to replace
them if you want to source your own.

- The `to_windows/*.(a|o)` files come from MSYS2's x86_64 mingw64
- `to_windows/from_x86_64_windows/ld.exe` comes from MSYS2's x86_64 mingw64
- `to_windows/from_x86_64_windows/llvm-windres.exe` comes from the `llvm-rc.exe`
  of MSYS2's LLVM package
- `to_windows/from_x86_64_linux/ld` comes from Ubuntu 24.04's build of x86_64
  mingw64
- `to_windows/from_x86_64_linux/llvm-windres` comes from Ubuntu 24.04's build of
  LLVM 18.1.8
- `to_windows/from_aarch64_macos/ld` comes from Homebrew's build of x86_64
  mingw64
- `to_windows/from_x86_64_linux/llvm-rc` comes from Homebrew's build of LLVM
  18.1.8
