# Setup

In order to build Obliteration from source make sure you have the following prerequisites.

## All platforms

- [Git](https://git-scm.com)
  - On Windows make sure you have `Run Git from the Windows Command Prompt` selected when installing
  - On Linux it is likely your distro already provided a package for this
  - On macOS you can install from Homebrew
- [Rust on the latest stable channel](https://www.rust-lang.org/tools/install)
  - Make sure you install using `rustup`
  - On Linux your distro may provide a package for this
  - On macOS you can install from Homebrew
- [Project](https://github.com/ultimaweapon/project)
  - You can install with `cargo install project`

## Windows

- [Visual Studio 2022](https://visualstudio.microsoft.com/vs)
  - Rust installer should already install this for you so you should not need to install this manually
  - Community edition are free for open-source project
  - `Desktop development with C++` workload is required
- [Windows Terminal](https://aka.ms/terminal)
  - You can use a classic `Command Prompt` but make sure you enable [ANSI escape sequences](https://stackoverflow.com/q/16755142/1829232)

## AArch64

- Rust on the latest nightly channel
  - You can install with `rustup toolchain install nightly`
- Rust std source for nightly
  - You can install with `rustup component add rust-src --toolchain nightly`
- Rust `aarch64-unknown-none-softfloat` target
  - You can install with `rustup target add aarch64-unknown-none-softfloat`
