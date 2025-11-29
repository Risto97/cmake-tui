# CMake TUI

A Terminal User Interface (TUI) for CMake configuration and build management, built with Rust and Ratatui.
Currently it is mostly a clone of `ccmake`, but I might add more features if I don't get bored by Rust.

Disclaimer:
This is just a fun project and me trying out Rust.
And what better use of Rust than making an utility for building real software written in C++.


## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/risto97/cmake-tui
cd cmake-tui

# Build and install
cargo install --path .
```

## Usage

```
# Run it in the build directory of your CMake project
cmake-tui
```

## License

This project is licensed under the LGPL-3.0 License - see the LICENSE file for details.
