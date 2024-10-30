# g-win

[![Crates.io](https://img.shields.io/crates/v/g-win.svg)](https://crates.io/crates/g-win)
[![Documentation](https://docs.rs/g-win/badge.svg)](https://docs.rs/g-win)
[![License](https://img.shields.io/crates/l/g-win.svg)](https://github.com/mj10021/g-win/blob/main/LICENSE)

**g-win** is a flexible and robust G-code parsing crate for Rust, built with the [Winnow](https://crates.io/crates/winnow) parsing library. It aims to accommodate as many flavors of G-code as possible by preserving unrecognized commands, ensuring compatibility across different machines and custom implementations.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [Parsing G-code](#parsing-g-code)
  - [Handling Unrecognized Commands](#handling-unrecognized-commands)
- [Examples](#examples)
- [Contributing](#contributing)
- [License](#license)

## Features

- **Extensible:** Built on winnow, g-win can be extended to parse any gcode flavor.
- **Preserves Unrecognized Commands:** Stores any unrecognized or custom commands as strings in place.
- **Easy Integration:** Simple API designed for ease of use in Rust applications.

## Installation

Add `g-win` to your `Cargo.toml`:

```toml
[dependencies]
g-win = "0.1.0"
```

Then, run:

```bash
cargo build
```

## Usage

### Parsing G-code

```rust
use g_win::GCodeModel;

fn main() {
    let gcode = "
        G21 ; Set units to millimeters
        G90 ; Absolute positioning
        M107 ; Fan Off
        G28 ; Home
        G1 Z15.0 F9000 ; Move Z Axis up
        MCustomCommand ; This is a custom command
    ";

    let gcode: GCodeModel = gcode.parse().expect("failed to parse");
    println!("{:?}", gcode);
}
```

### Handling Unrecognized Commands

`g-win` stores unrecognized or custom commands as `Command::Raw(String)`, preserving their original content.
You can use a map to process them as needed.

```rust
use g_win::{GCodeModel, Command};

fn main() {
    let gcode = "G2 X2.89 Y6.0 R1.0 ; Arc move not currently parsed by g-win";
    let mut gcode: GCodeModel = gcode.parse().expect("failed to parse");
    gcode.lines = gcode.lines.iter().map()
}
```

## Examples

- **Basic Parsing:** Parse standard G-code commands.
- **Custom Commands:** Handle and preserve custom or machine-specific commands.
- **Error Handling:** Examples showing how to handle parsing errors gracefully.

For more examples, check out the [examples](https://github.com/yourusername/g-win/tree/main/examples) directory in the repository.

## Contributing

Contributions are welcome! Please submit a pull request or open an issue for suggestions and improvements.

1. Fork the repository.
2. Create a new branch: `git checkout -b feature/your-feature-name`.
3. Commit your changes: `git commit -am 'Add some feature'`.
4. Push to the branch: `git push origin feature/your-feature-name`.
5. Open a pull request.

## License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/yourusername/g-win/blob/main/LICENSE) file for details.

---

*Happy Coding!*