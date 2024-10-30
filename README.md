# g-win

  

[![Crates.io](https://img.shields.io/crates/v/g-win.svg)](https://crates.io/crates/g-win)

[![Documentation](https://docs.rs/g-win/badge.svg)](https://docs.rs/g-win)

[![License](https://img.shields.io/crates/l/g-win.svg)](https://github.com/mj10021/g-win/blob/main/LICENSE)

  

**`g-win`** is a G-code parsing crate for Rust, built with [`winnow`](https://crates.io/crates/winnow). It aims to maximize compatibility by preserving unrecognized commands for later processing, ensuring compatibility across environments and handling of features like macros and templating.

  

## Table of Contents

  

- [Features](#features)

- [Installation](#installation)

- [Usage](#usage)

- [Contributing](#contributing)

- [License](#license)

  

## Design Goals

  


-  **Preserves Unrecognized Commands:** Stores any unrecognized or custom commands as strings in place.

-  **Custom Command Handling:** Easily add rules to parse any command.

- **Lightweight:** Minimal API designed to streamline implementation.

  

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

  All G-code file information is stored in the `GCodeModel` struct.  The parser is implemented through the `FromStr` trait, returning a result of the type `Result<GCodeModel, GCodeParseError>`.
```rust

use  g_win::GCodeModel;

let  gcode = "
	G21 ; Set units to millimeters
	G90 ; Absolute positioning
	M107 ; Fan Off
	G28 ; Home
	G1 Z15.0 F9000 ; Move Z Axis up
	MCustomCommand ; This is a custom command
	";
	
let  gcode: GCodeModel = gcode.parse().expect("failed to parse");
println!("{:?}", gcode);

```

  

### Handling Unrecognized Commands

`g-win` stores unrecognized or custom commands as `Command::Raw(String)`, preserving their original content.


## Contributing

  

Contributions are welcome! Please submit a pull request or open an issue for suggestions and improvements.

  

1. Fork the repository.

2. Create a new branch: `git checkout -b feature/your-feature-name`.

3. Commit your changes: `git commit 'Add some feature'`.

4. Push to the branch: `git push origin feature/your-feature-name`.

5. Open a pull request.

  

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
