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
- [Python 3](https://www.python.org)
  - On Windows make sure you have `Add Python to PATH` selected when installing
  - On Linux it is likely your distro already provided a package for this
  - On macOS you can install latest version from Homebrew

## Windows

- [Visual Studio 2022](https://visualstudio.microsoft.com/vs)
  - Rust installer should already install this for you so you should not need to install this manually
  - Community edition are free for open-source project
  - `Desktop development with C++` workload is required
- [Windows Terminal](https://aka.ms/terminal)
  - You can use a classic `Command Prompt` but make sure you enable [ANSI escape sequences](https://stackoverflow.com/q/16755142/1829232)
