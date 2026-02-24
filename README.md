# PackageHeroMock

PackageHeroMock is a small Rust-based virtual serial device that simulates PackageHERO® industrial scale and laser measurement hardware. It exposes a pseudo-tty (by default at `/tmp/ttyPackageHero`) that client software can open to receive simulated scale and laser data.

## Features

- Simulates a weight scale: pressing ENTER in the simulator sends a randomized weight reading.
- Simulates a laser distance sensor: pressing `L` then ENTER sends a randomized distance measurement.
- Provides simple command responses for a few control messages sent by the client.

## Prerequisites

- Rust and Cargo (recommended: recent stable toolchain)
- Linux system with pseudo-tty support (the simulator uses a PTY)

## Build

From the repository root run:

```bash
cargo build --release
```

## Run

Run the simulator locally with:

```bash
cargo run --release
```

When started the program creates a symlink at `/tmp/ttyPackageHero` that points to the slave side of the created PTY. If the link already exists it will be replaced.

The simulator prints basic usage instructions to its stdout. Typical interaction is:

- Press ENTER to send a weight (scale) reading.
- Type `L` and press ENTER to send a laser distance measurement.

Client programs can open `/tmp/ttyPackageHero` as a serial device (e.g. via a serial library or a terminal program) to receive the simulated device data.

## Protocol notes

- Scale responses are ASCII strings terminated with CRLF (for example `A00A0000000123\r\n`).
- Laser messages and acknowledgements are sent as binary frames starting with `0xAA` and ending with `0x0D 0x0A`.
- The simulator also provides simple responses to a few command patterns used by client software; see `src/main.rs` for the exact behavior.

## Development

- The main simulator implementation is in `src/main.rs`.
- Feel free to modify the behavior (random ranges, response formats) to better match your target hardware.

## Troubleshooting

- If `/tmp/ttyPackageHero` is not present after startup, check program output for errors related to PTY creation or file permissions.
- Ensure no other process is holding the same symlink path; the simulator removes and recreates the link on startup.

## License

This repository does not include a license file. Add an appropriate license if you plan to publish or share this project on GitHub.
