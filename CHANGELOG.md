# Changelog

## [unreleased]

- Update toml-related crates to newest

## [0.17.1] - 2025-06-30

### Changed

- On high-end CPUs on unix platforms, the "Maximum number of open files" would likely be hit, `mdns-scanner` now dynamically requests to raise that limit, or limits socket I/O if needed.

## [0.17.0] - 2025-06-29

### Added

- Dynamic resource scaling. Thread usage and TUI refresh rate now adapt to the host's [available parallelism](https://doc.rust-lang.org/std/thread/fn.available_parallelism.html), improving performance on high-end CPUs and ensuring efficiency on low-power devices.

### Fixed

- Rare crash if an DNS-SD service was identified at an IP before a host was identified at the same IP. Can occur if scanning an (exclusively) Ipv6 network interface, as the Ipv6 scanning support is only partial at this point.
- False positives when using the native ping binary on windows (fallback if insufficient permissions for raw socket use)

## [0.16.1] - 2025-06-29

### Changed

- Remove `jemalloc` from the prebuilt binaries for all platforms except for x86_64 Linx and MacOS. Please report any issues, `jemalloc` will be completely removed if more issues pop up.

## [0.16.0] - 2025-06-28

### Changed

- Replace `dns-parser` with `hickory-proto` for all low level DNS operations.
- Reworked `DNS-SD` service resolution for massive performance gain.

## [0.15.1] - 2025-06-28

### Changed

- Improve the **refresh** action, avoiding showing any stale scanning information. Abusing the refresh action (e.g. by holding down CTRL+R) can cause it to enter a state where new IP info is never displayed, in this case it's fixed by freshing once more.
- (Unix) Simpler and more efficient ICMP host up check when using raw sockets

## [0.15.0] - 2025-06-23

### Added

- The `scan.tcp_ports` setting now determines which TCP ports are scanned to ascertain host reachability.

### Changed

- If an IP address was previously found to be reachable, the status is now re-verified using the exact method (exact TCP port connection or ICMP ping) that initially confirmed its reachability.

## [0.14.0] - 2025-06-23

### Added

- the `ui.log_limit` setting controls the maximum number of logs to store before the oldest logs are dropped

### Changed

- Flip table/log panes, the log now appears on the bottom which seems better
- Adjust search box to take up less space, and scale with the input if it's enough to fill the box

## [0.13.2] - 2025-06-22

### Fixed

- Erroneous definition of the `interfaces.ignore_patterns` config option

## [0.13.1] - 2025-06-22

### Changed

- Corrected label `Compact Output` to `Compact UI` to better reflect what it does
- Don't accept missing values from persisted config
- Tweak config layout for better readability and extensibility

## [0.13.0] - 2025-06-22

### Added

- config file, dump the default config to the terminal with the command `dump-default-config`
- config key `hide_bare_ips` hides any IPs with no resolved hostnames or services
- Pressing `Ctrl+C` opens a config window for editing the config during a session, currently limited to toggling boolean values
- Pressing `Ctrl+R` refreshes all scanning state

### Changed

- Significantly reduce binary size by scrutinizing dependencies
- To simplify installation from source with the default profile, `jemalloc` is feature-gated. While it's included in prebuilt binaries, users installing from source must explicitly enable the `jemalloc` feature to use it instead of the system allocator, i.e.: `cargo install --git https://github.com/CramBL/mdns-scanner mdns-scanner --features jemalloc`

## [0.12.1] - 2025-06-15

### Changed

- Update docs
- Use `parking_lot` for synchronization primitives

## [0.12.0] - 2025-06-15

### Added

- The `update` command allows updating `mdns-scanner` (and downgrading if needed), only available if `mdns-scanner` is installed through the (new) install script
- support for `aarch64-pc-windows-msvc`
- support for `i686-pc-windows-msvc`
- support for `powerpc64-unknown-linux-gnu`
- support for `powerpc64le-unknown-linux-gnu`
- support for `riscv64gc-unknown-linux-musl`
- support for `s390x-unknown-linux-gnu`
- support for `i686-unknown-linux-musl`

### Fixed

- Fix and occasional thread panic that could happen while the app is shutting down

## [0.11.1] - 2025-06-15

### Changed

- Use `jemalloc` on most unix and unix-like systems (using `mimalloc` on windows).

## [0.11.0] - 2025-06-14

### Added

- Add CLI option `ip-check-timeout-ms` for setting the upper time limit for checking if a host is up on an IP
- Add CLI option `ping-timeout-ms` for setting how long to wait for echo replies
- Add CLI option `tcp-port-timeout-ms` for setting how long to wait before timing out a TCP connection on each individual port

### Changed

- Tweaked CLI style to have generally more readable colors.
- Migrate to workspace and update some dependencies
- Reduce binary size by ~3%
- Add a max waiting time for either ICMP or TCP IP checking to finish

### Fixed

- Sending ICMP Echo Requests via raw sockets on windows would hang if the host was unreachable

## [0.10.0]

### Added

- Replace global allocator with mimalloc for windows and macos targets
- Compact mode `-c` or `--compact` hides the footer that displays key bindings (and version)

### Changed

- Fix lint from clippy v1.87.0
- Update dependencies

### Fixed

- Fix crash in certain window resolutions when log pane size reaches minimum.

## [0.9.1]

### Fixed

- Failed to identify running network interfaces on windows
- Incorrect usage of native ping binary on windows when falling back from programmatically sending ICMP packets

## [0.9.0]

### Added

- Add `--no-dns-sd` for disabling service discovery

### Changed

- Ensure a narrow window width doesn't hide IP, or hostnames. Instead, the `Services` column is gradually cropped off.

## [0.8.0]

### Added

- Add Service discovery: now resolves and shows local DNS-SD services, including TXT metadata.

## [0.7.0]

### Changed

- Keyboard shortcut indicator for quit changed from 'q' to 'Q' (both are valid though)
- rename command-line option `ignore-re-iface` to `iface-ignore-re`
- Interfaces that resemble docker networks are excluded by default, can be included with `--iface-include-docker`
- Improve CLI help description

### Internal

- Remove dependency on `get_if_addrs`
- Add `dependabot.yml`
- Add test coverage tracking
- Add typo-checking
- Update dependencies

## [0.6.1]

### Fixed

- Help footer showed incorrect key shortcut for quitting

## [0.6.0]

### Added

- Discovered hosts are colored green for 10s when they are first discovered or whenever they are updated (e.g. if a new hostname is found to be associated with the IP)
- Discovered hosts that become unreachable are colored red

## [0.5.0]

### Added

- Add minimal CLI with version and help flags
- Add the `ignore-re-iface` option which can be used (multiple times) to ignore network interfaces based on regular expression pattern matching

### Changed

- Decreasing verbosity is now done with `g` instead of `c`

### Fixed

- Fix issues with wrongly resolved hostnames from misinterpreted PTR records.

## [0.4.1]

### Changed

- Changed verbosity of some log messages

### Fixed

- Fix pane resizing caused crash if log pane got size 1 in QTerminal
- Fix hostnames would contain duplicates under certain conditions
- Remove a bad mDNS query question that would show up in the log as an error with the text `query type 1388 is invalid`
- Fix mDNS queries using invalid query ID, now compliant with RFC6762

## [0.4.0]

### Added

- Footer with key mapping info
- Ability to adjust proportional size of each pane with `+/-` keys

### CI

- Add code scanning via `osv-scanner` & `cargo-audit`.
- Add check for minimum support rust version

## [0.3.0]

### Added

- Add scrollbar and navigation for log pane

## [0.2.0]

### Changed

- Add search box to filter IP info table
- Add/fix ip info pane scrollbar
- Limit stored logs to 1000

## [0.1.0]

- Initial release
