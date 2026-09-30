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

The [v0.1.0 GitHub release](https://github.com/jeeftor/tnt/releases/tag/v0.1.0) contains a prebuilt, static Linux x86_64 executable. You can place it in a subdirectory of an existing data-pool dataset if your account can write there and the mount permits execution. Replace `YOUR_POOL` and `EXISTING_DATASET` with the actual names:

```sh
findmnt -T /mnt/YOUR_POOL/EXISTING_DATASET -o TARGET,OPTIONS
test -w /mnt/YOUR_POOL/EXISTING_DATASET && echo writable
mkdir -p /mnt/YOUR_POOL/EXISTING_DATASET/tnt
cd /mnt/YOUR_POOL/EXISTING_DATASET/tnt
```

The `findmnt` output must not include `noexec`, and the write check must print `writable`. Do not change Exec or permissions on an existing shared or application dataset solely for TNT. If no suitable dataset exists, create a dedicated one through the [TrueNAS command line API client](https://www.truenas.com/docs/scale/api/). Replace `YOUR_POOL` with a data pool name (not `boot-pool`) and `YOUR_USER` with your SSH username:

```sh
zpool list -H -o name
sudo midclt call pool.dataset.create '{"name":"YOUR_POOL/tnt","share_type":"GENERIC","exec":"ON"}'
sudo midclt call -j filesystem.setperm '{"path":"/mnt/YOUR_POOL/tnt","user":"YOUR_USER","mode":"700"}'
findmnt -T /mnt/YOUR_POOL/tnt -o TARGET,OPTIONS
cd /mnt/YOUR_POOL/tnt
```

Run the create command only if `YOUR_POOL/tnt` does not already exist. The dataset uses space as files are added; no quota or reservation is needed. `findmnt` should show the new dataset mount without `noexec`.

From either chosen directory, download and run TNT:

```sh
curl -fLO https://github.com/jeeftor/tnt/releases/download/v0.1.0/tnt-linux-amd64
curl -fLO https://github.com/jeeftor/tnt/releases/download/v0.1.0/SHA256SUMS
sha256sum -c SHA256SUMS
chmod +x tnt-linux-amd64
./tnt-linux-amd64
```

Do not install TNT in `/usr/bin` or `/usr/local/bin`; TrueNAS manages its operating system files. The administrator home directory can be mounted `noexec`, which prevents running a binary there even after `chmod +x` or with `sudo`. Check a location with `findmnt -T /path/to/tnt-linux-amd64 -o TARGET,OPTIONS`. The `dist/` directory in this repository contains the same executable and checksum if you prefer to copy them to the dataset. On macOS, use `shasum -a 256 -c SHA256SUMS` to verify those files.

To build instead, run this on a Linux host with Rust installed:

```sh
cargo build --release
```

The resulting executable is `target/release/tnt`. Build on Linux x86_64 if you need the documented TrueNAS deployment architecture; a build on macOS produces a macOS executable.

## Use

```sh
./tnt-linux-amd64
./tnt-linux-amd64 --interval 2
./tnt-linux-amd64 --help
```

The default sample interval is five seconds. `--interval` accepts whole seconds from 1 through 60. Use ↑/↓ or `j`/`k` to select a sensor and see its larger history chart. Press `q`, Esc, or Ctrl+C to quit.

Each sensor retains up to 120 samples in memory while it remains in the current sensor output. The displayed peak is the highest value seen for that sensor since it appeared; it is not limited to those 120 samples. History and peak values reset when the program restarts or a sensor disappears and later returns.

Temperature colors use the hardware critical value when available, otherwise the hardware maximum. Yellow begins at 85% of that limit and red at the limit. Without a reported limit, temperatures stay green. Fan readings stay cyan. Graph scaling uses the reported limit when present, otherwise the observed peak.

## Troubleshooting

- If TNT fails with `Permission denied`, check `findmnt -T /path/to/tnt-linux-amd64 -o TARGET,OPTIONS` for `noexec`. Run it from a pool dataset with Exec set to On. Also check that the binary has execute permission and matches the host architecture.
- If it reports that `sensors -j` cannot be started, run `sensors -j` on the same host and check that the command is in `PATH`.
- If it shows zero sensors, inspect `sensors -j` for numeric `fan*_input` or `temp*_input` values. Other fields are ignored.
- If a sample fails, the footer shows the collection or JSON error. The last successful readings remain visible, and the footer shows how old they are.

Some hosts report unreadable PWM control values on standard error while still producing usable JSON on standard output. `tnt` reads the JSON and never writes PWM settings.

## Coverage and limits

The first version reads the temperatures and fan speeds exposed by `lm-sensors`. It groups `drivetemp` chips as Drives and `coretemp` chips or features named `CPU` as CPU; other temperatures appear under System. These names are display groups, not a hardware discovery guarantee. Drive names from `smartctl`, HBA temperatures from `storcli`, and custom thresholds in `temps.yaml` are not supported.

Sampling uses an external `sensors -j` process. There is currently no timeout for that process, so a stuck command can stop updates until it exits. The monitor runs on Linux; a non-Linux build can display `--help` but cannot start the monitor.

## Development checks

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps --document-private-items
```

The unit tests cover sample JSON parsing and basic terminal rendering. They do not verify a live TrueNAS host or the bundled Linux executable.
The documentation command generates browsable source API docs at `target/doc/tnt/index.html`. The private-items flag includes this small binary's internal functions and types.
