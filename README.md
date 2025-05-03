<div align=right>Table of Contents↗️</div>

<h1 align=center>MDNS Scanner

<code>mdns-scanner</code>

</h1>

<div align="center">
  <a href="https://github.com/CramBL/mdns-scanner/releases" title="Latest Stable GitHub Release">
      <img src="https://img.shields.io/github/release/CramBL/mdns-scanner/all.svg?style=flat&logo=github&logoColor=white&colorB=blue&label=Latest Release" alt="GitHub release"></a>
    <a href=https://github.com/CramBL/mdns-scanner/actions>
    <img src=https://github.com/CramBL/mdns-scanner/actions/workflows/CI.yml/badge.svg alt="CI status">
  </a>
    <a href=https://codecov.io/github/CramBL/mdns-scanner>
    <img src=https://codecov.io/github/CramBL/mdns-scanner/graph/badge.svg?token=TxW5dzMN0w alt=codecov>
  </a>
    <img src="https://img.shields.io/badge/-Windows-6E46A2.svg?style=flat&logo=windows-11&logoColor=white" alt="Windows" title="Supported Platform: Windows">&thinsp;
    <img src="https://img.shields.io/badge/-Linux-9C2A91.svg?style=flat&logo=linux&logoColor=white" alt="Linux" title="Supported Platform: Linux">&thinsp;
    <img src="https://img.shields.io/badge/-macOS-red.svg?style=flat&logo=apple&logoColor=white" alt="macOS" title="Supported Platform: macOS">
</div>

## Purpose

Scan a network and create a list of IPs and associated hostnames, including mDNS hostnames and other aliases.

## Install

### Prebuilt binaries

Prebuilt binaries for Linux, MacOS, and Windows can be found on [the releases page](https://github.com/CramBL/mdns-scanner/releases).

```console
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/CramBL/mdns-scanner/trunk/scripts/install.sh \
    | bash -s -- --to ~/bin
```

### With `cargo`

```console
cargo install mdns-scanner
```

### Quickstart

Simply run it.

`mdns-scanner` will start scanning any non-loopback network interfaces for IPs with a host on the other end, and resolve the hostnames for those IPs.

> [!TIP]
> Inform your resident sys admin that you're about to run hundreds of IP scans per second.

### Runtime dependencies

None.

## Architecture

![architecture](/docs/architecture.svg)
