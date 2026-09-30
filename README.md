<p align="center">
  <img src="assets/tnt-logo.svg" alt="TNT — TrueNAS Temps, a read-only terminal sensor monitor" width="720">
</p>

# TNT — TrueNAS Temps

Watch temperatures and fan speeds from a TrueNAS SCALE terminal. **TNT is read-only:** it samples `sensors -j`, shows the latest values and their history, and never changes fan settings.

## Quick start

You need a Linux x86_64 host, an interactive terminal (console or SSH), and `sensors` from `lm-sensors` in `PATH`. Confirm that your host reports readings:

```sh
sensors -j
```

Then, from a copy of this repository on the Linux host:

```sh
cd dist
sha256sum -c SHA256SUMS
./tnt-linux-amd64
```

Use `./tnt-linux-amd64 --interval 2` to sample every two seconds. The default is five seconds; accepted intervals are whole seconds from 1 through 60. The bundled executable is a static Linux x86_64 binary. It must be on a filesystem that permits execution.

If you verify the files on macOS before copying them to the NAS, use `shasum -a 256 -c SHA256SUMS` inside `dist/`. A macOS build cannot run the monitor; only `--help` works outside Linux.

## Controls and display

| Key | Action |
| --- | --- |
| ↑ / ↓ or `k` / `j` | Select a sensor and show its larger history chart |
| `q`, Esc, or `Ctrl+C` | Quit |

The list groups fan, CPU, system, and drive readings. Each row shows a current value and a small relative history chart. The larger chart uses the hardware limit when one is reported, or the observed peak with some headroom otherwise.

TNT keeps up to 120 samples per sensor in memory. The displayed peak covers that sensor's full lifetime in the current run, even after older history samples drop off. If a sensor disappears and returns, its history and peak restart.

Temperature readings turn yellow at 85% of their reported critical or maximum value, and red at that value. Without a reported limit, they remain green. Fan readings stay cyan.

## Build from source

On Linux with Rust installed:

```sh
cargo build --release
./target/release/tnt
```

Build on Linux x86_64 for the documented TrueNAS deployment target. Building on macOS produces a macOS executable rather than the Linux binary in `dist/`.

## Sensor coverage

TNT reads finite numeric `fan*_input` and `temp*_input` fields from `sensors -j`. What appears depends on the hardware and drivers available to `lm-sensors`. It places `drivetemp` chips under Drives and `coretemp` chips or features named `CPU` under CPU; other temperatures appear under System. These are display groups, not hardware identification.

Drive names from `smartctl`, HBA temperatures from `storcli`, and custom thresholds in `temps.yaml` are outside the current version's scope. TNT does not read or write PWM controls.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Binary will not start | Confirm the host is Linux x86_64 and the file is on an executable filesystem; a `noexec` mount cannot run it directly. |
| `sensors -j` cannot start | Run `sensors -j` on the same host and check that it is in `PATH`. |
| Zero sensors appear | Look for numeric `fan*_input` or `temp*_input` values in `sensors -j`; other fields are ignored. |
| Readings stop updating | Check the footer for a collection or JSON error and the age of the last successful sample. A stuck `sensors -j` process currently has no timeout. |

Some hosts print unreadable PWM errors to standard error while producing usable JSON on standard output. TNT uses the valid JSON and does not write PWM settings.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps --document-private-items
```

The documentation command writes browsable source docs to `target/doc/tnt/index.html`. The tests cover sample JSON parsing and basic terminal rendering; they do not establish live TrueNAS behavior or verify the bundled binary on a NAS.
