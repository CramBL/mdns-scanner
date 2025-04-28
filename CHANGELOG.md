# Changelog

## [unreleased]

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
