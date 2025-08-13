# Changelog

## [unreleased]

## [0.22.3] - 2025-08-13

### Fixed

- [#143](https://github.com/CramBL/mdns-scanner/issues/143): Log error instead of crashing on partially resolved DNS-SD service with no associated IP
- Update `slab` from `0.4.10` to `0.4.11` to fix out-of-bounds access [RUSTSEC-2025-0047](https://rustsec.org/advisories/RUSTSEC-2025-0047)

### Dependencies

- `tokio`: 1.47.0 → 1.47.1 ([#138](https://github.com/CramBL/mdns-scanner/pull/138))
- `toml_edit`: 0.23.2 → 0.23.3 ([#138](https://github.com/CramBL/mdns-scanner/pull/138))
- `toml`: 0.9.3 → 0.9.5 ([#138](https://github.com/CramBL/mdns-scanner/pull/138))
- `crate-ci/typos`: 1.34.0 → 1.35.1 ([#139](https://github.com/CramBL/mdns-scanner/pull/139))
- `cargo-bins/cargo-binstall`: 1.14.2 → 1.14.3 ([#139](https://github.com/CramBL/mdns-scanner/pull/139))
- `dns-lookup`: 2.0.4 → 2.1.0 ([#142](https://github.com/CramBL/mdns-scanner/pull/142))
- `clap`: 4.5.41 → 4.5.44 ([#142](https://github.com/CramBL/mdns-scanner/pull/142))
- `thiserror`: 2.0.12 → 2.0.13 ([#142](https://github.com/CramBL/mdns-scanner/pull/142))

## [0.22.2] - 2025-08-01

- Migrate back to the original `cargo-dist` (dist) for packaging and releasing as it's now maintained again
- Fix homebrew publish job (broke in the astral fork of `cargo-dist`)

## [0.22.1] - 2025-07-30

- Publish to homebrew
- Fix incorrect width of `Services` column.

### Dependencies

- `ringbuffer`: 0.15.0 → 0.16.0

## [0.22.0] - 2025-07-29

### Added

- IP/Hostname entries are now merged if they share an IP/Hostname mapping, in practice that means that a host that is discovered both via IPv4 and IPv6 appears as a single entry in the table
- Searching also fuzzy-searches the `services` column

### Fixed

- Store both ipv4 and ipv6 for DNS-SD services and ensure all IPs get associated to discovered services, regardless of order of received DNS packets
- Fix the IP column width to the size of an IPv6
- Fix RTT stats not being collected for a host that was first discovered via DNS-SD

### Dependencies

- `tokio`: 1.46.1 → 1.47.0 ([#125](https://github.com/CramBL/mdns-scanner/pull/125))
- `socket2`: 0.5.10 → 0.6.0 ([#125](https://github.com/CramBL/mdns-scanner/pull/125))
- `toml`: 0.9.2 → 0.9.3 ([#125](https://github.com/CramBL/mdns-scanner/pull/125))
- `cargo-bins/cargo-binstall`: 1.14.1 → 1.14.2 ([#124](https://github.com/CramBL/mdns-scanner/pull/124))

## [0.21.0] - 2025-07-24

- Unescape escaped UTF-8 in domain names when displaying them, according to [RFC 1035 section 5.1](https://datatracker.ietf.org/doc/html/rfc1035#section-5.1)
- Remove redundant DNS-SD name when the same name is later discovered to be a hostname associated with the same IP that the service is also associated with
- Add `log_level` configuration key for specifying the log level at startup
- Wrap lines when displaying properties of a DNS-SD service
- Differentiate timeout errors (logged as `debug`) from other errors when performing mDNS lookup
- Improve some log messages and reduce verbosity of log statements from received DNS records from `info` to `debug`

### Dependencies

- `axoupdater`: 0.9.0 → 0.9.1 ([#112](https://github.com/CramBL/mdns-scanner/pull/112))
- `strum`: 0.27.1 → 0.27.2 ([#112](https://github.com/CramBL/mdns-scanner/pull/112))
- `toml_edit`: 0.23.1 → 0.23.2 ([#112](https://github.com/CramBL/mdns-scanner/pull/112))

## [0.20.0] - 2025-07-21

- Optimize logging implementation by avoiding string allocating and formatting for messages that are below the current verbosity
- Remove the verbosity indicator from log lines (e.g. `[I]` for `info`)
- Somehow reduce binary size by ~100KiB after integrating the custom logger with the `log` facade

## [0.19.0] - 2025-07-20

- Add popup when selecting a discovered IP from the table pane, the popup shows additional information such as RTT stats and more.
- Log the time to get a reply from a host via either ping or TCP connection
- Allow configuring the number of `io_threads` used for network scans (or use the default 'dynamic' setting). See the config editor or the [default_config.toml](./docs/default_config.toml) for more.
- Deduplicate list values before assignment when editing lists in the config editor

## [0.18.0] - 2025-07-18

### Config Editor

- Enabled editing of all configuration values via the config editor popup.
- Added inline descriptions for the currently selected configuration key.
- Reworked the config editor layout.

### Dependencies

- Updated TOML-related crates to their latest versions.
- Refreshed other project dependencies with `cargo update.

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
