# MDNS Scanner

## Purpose

Scan a network and create a list of IPs and associated hostnames, including mDNS hostnames and other aliases.

## Install

### Prebuilt binaries

Prebuilt binaries for Linux, MacOS, and Windows can be found on [the releases page](https://github.com/CramBL/mdns-scanner/releases).

```console
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/CramBL/mdns-scanner/trunk/scripts/install.sh \
    | bash -s -- --to ~/bin
```

### From Source

```console
cargo install --git https://github.com/CramBL/mdns-scanner.git
```

### Runtime dependencies

None.

## Architecture

![architecture](/docs/architecture.svg)
