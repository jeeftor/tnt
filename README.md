# TNT — TrueNAS Temps

`tnt` is a read-only terminal monitor for temperatures and fan speeds on TrueNAS SCALE. It samples the JSON output of `sensors -j` and shows current values and recent history. It does not change fan speeds or sensor settings.

## Requirements

- A Linux x86_64 host for the bundled binary, or a Linux host with Rust to build from source.
- `sensors` from `lm-sensors` available in `PATH`, with at least one temperature or fan reading in `sensors -j`.
- An interactive terminal, such as an SSH session or the local console.

Check the sensor source on the host before running `tnt`:

```sh
sensors -j
```

Only `fan*_input` and `temp*_input` numeric fields are displayed. Available readings depend on what the host exposes through `lm-sensors`.

## Install

The `dist/` directory contains a prebuilt, static Linux x86_64 executable. On the host where you downloaded the repository, verify the file from inside `dist/`:

```sh
cd dist
sha256sum -c SHA256SUMS
```

On macOS, use `shasum -a 256 -c SHA256SUMS` instead. Copy `tnt-linux-amd64` to an executable filesystem on your TrueNAS host and name it `tnt`. You can also run the file directly from any executable location. A file on a `noexec` filesystem cannot be started directly.

To build instead, run this on a Linux host with Rust installed:

```sh
cargo build --release
```

The resulting executable is `target/release/tnt`. Build on Linux x86_64 if you need the documented TrueNAS deployment architecture; a build on macOS produces a macOS executable.

## Use

```sh
tnt
tnt --interval 2
tnt --help
```

The default sample interval is five seconds. `--interval` accepts whole seconds from 1 through 60. Use ↑/↓ or `j`/`k` to select a sensor and see its larger history chart. Press `q`, Esc, or Ctrl+C to quit.

Each sensor retains up to 120 samples in memory while it remains in the current sensor output. The displayed peak is the highest value seen for that sensor since it appeared; it is not limited to those 120 samples. History and peak values reset when the program restarts or a sensor disappears and later returns.

Temperature colors use the hardware critical value when available, otherwise the hardware maximum. Yellow begins at 85% of that limit and red at the limit. Without a reported limit, temperatures stay green. Fan readings stay cyan. Graph scaling uses the reported limit when present, otherwise the observed peak.

## Troubleshooting

- If `tnt` cannot start, check that the binary matches the host architecture and is on an executable filesystem.
- If it reports that `sensors -j` cannot be started, run `sensors -j` on the same host and check that the command is in `PATH`.
- If it reports no usable readings, inspect `sensors -j` for numeric `fan*_input` or `temp*_input` values. Other fields are ignored.
- If a sample fails or exceeds the ten-second command timeout, the footer shows the error. The last successful readings remain visible, and the footer shows how old they are.

Some hosts report unreadable PWM control values on standard error while still producing usable JSON on standard output. `tnt` uses those readings even if the command exits unsuccessfully, and marks them as partial data in the footer. It never writes PWM settings.

## Coverage and limits

The first version reads the temperatures and fan speeds exposed by `lm-sensors`. It groups `drivetemp` chips as Drives and `coretemp` chips or features named `CPU` as CPU; other temperatures appear under System. These names are display groups, not a hardware discovery guarantee. Drive names from `smartctl`, HBA temperatures from `storcli`, and custom thresholds in `temps.yaml` are not supported.

Sampling uses an external `sensors -j` process with a ten-second timeout. The monitor runs on Linux; a non-Linux build can display `--help` but cannot start the monitor.

## Development checks

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps --document-private-items
```

The unit tests cover sample JSON parsing, collector failures, and basic terminal rendering. They do not verify a live TrueNAS host or the bundled Linux executable.
The documentation command generates browsable source API docs at `target/doc/tnt/index.html`. The private-items flag includes this small binary's internal functions and types.
