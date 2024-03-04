# Gravio Manager

Client: Manages installations of Studio, Hubkit on a (local) machine Server:
Hosts versioned Gravio binaries

## Requirements (Usage)

### Windows

- Powershell 5+

## Requirements (Building)

- Rust 1.76+

# Setup

Run the setup script, which installs necessary post-build tool for generating
SBOMs and checksums

```cmd
setup.ps1
```

# Building

Run the build script, which also takes care of post-build steps

```cmd
build.ps1
```

# Examples

```bash
$ .\target\release\graviomanager.exe -h

Manages Asteria products on a machine

Usage: graviomanager.exe [COMMAND]

Commands:
  list       Lists installation candidates
  uninstall  Uninstalls the candidate
  install    Installs the [candidate] with optional [version]
  cache      Clears the cache of all matching criteria, or all of it, if nothing specified
  installed  Lists items that are installed on this machine
  config     Deals with the configuration
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Show installed items

```cmd
$ .\target\release\graviomanager.exe installed

┌──────────────┬────────────┬─────────────────────────────────────────────────────────────┐
│     Name     │  Version   │                         Identifier                          │
├──────────────┼────────────┼─────────────────────────────────────────────────────────────┤
│ GravioStudio │ 5.2.4670.0 │ InfoteriaPte.Ltd.GravioStudio_5.2.4670.0_x64__mrnz526z5qc9p │
│ HubKit       │ 5.2.1.7055 │ {F695BACF-2021-48C7-8283-90341BB01360}                      │
└──────────────┴────────────┴─────────────────────────────────────────────────────────────┘
```

## Show available items to install

```cmd
$ .\target\release\graviomanager.exe list --show-installed

┌───────────────┬────────────┬───────────────────────┬─────────────────────────┬───────────┐
│     Name      │  Version   │      Identifier       │         Flavor          │ Installed │
├───────────────┼────────────┼───────────────────────┼─────────────────────────┼───────────┤
│ GravioStudio  │ 5.2.4683   │ develop               │ WindowsAppStore         │           │
│ GravioStudio  │ 5.2.4683   │ develop               │ Sideloading             │           │
│ GravioStudio  │ 5.2.4682   │ reorg_login           │ WindowsAppStore         │           │
│ GravioStudio  │ 5.2.4682   │ reorg_login           │ Sideloading             │           │
│ GravioStudio  │ 5.2.4679   │ backport_fix_qos      │ WindowsAppStore         │           │
│ GravioStudio  │ 5.2.4679   │ backport_fix_qos      │ Sideloading             │           │
│ GravioStudio  │ 5.2.4674   │ master                │ WindowsAppStore         │           │
│ GravioStudio  │ 5.2.4674   │ master                │ Sideloading             │           │
│ GravioStudio  │ 5.2.4670.0 │ --                    │ --                      │ true      │
│ HandbookX     │ 1.0.1660.0 │ not_logged_in         │ Windows                 │           │
│ HandbookX     │ 1.0.1660.0 │ not_logged_in         │ Sideloading             │           │
│ HandbookX     │ 1.0.1659.0 │ develop               │ Windows                 │           │
│ HandbookX     │ 1.0.1659.0 │ develop               │ Sideloading             │           │
│ HandbookX     │ 1.0.1658.0 │ master                │ Windows                 │           │
│ HandbookX     │ 1.0.1658.0 │ master                │ Sideloading             │           │
│ HubKit        │ 5.2.1.7055 │ --                    │ --                      │ true      │
│ HubKit        │ 5.2.1-7061 │ zigbee_dongle         │ WindowsHubkit           │           │
│ HubKit        │ 5.2.1-7059 │ master                │ WindowsHubkit           │           │
│ HubKit        │ 5.2.1-7055 │ develop               │ WindowsHubkit           │           │
│ HubKit        │ 5.2.1-7053 │ experimental_endpoint │ WindowsHubkit           │           │
│ UpdateManager │ 5.2.400    │ develop               │ WindowsUpdateManagerExe │           │
│ UpdateManager │ 5.2.398    │ master                │ WindowsUpdateManagerExe │           │
└───────────────┴────────────┴───────────────────────┴─────────────────────────┴───────────┘
```

## Uninstall a product

```cmd
$ .\target\release\graviomanager.exe uninstall graviostudio

Looking to uninstall an item: graviostudio
Found uninstallation target. Attempting to uninstall GravioStudio                                                                                           
Successfully uninstalled GravioStudio
```

## Install a product

```
 $ .\target\release\graviomanager.exe install graviostudio develop

 
Installing graviostudio@develop, flavor WindowsAppStore
A candidate for installation has been found in the local cache, but since the version was unspecified it may be oudated. Would you like to check the remote repositories for updated versions? [y/N]
graviostudio, 5.2.4670
y
Will search for more recent versions, and will use this cached item as fallback
Found a version on the server for this identifier that is greater than the one in cache (cached: 5.2.4670, found: 5.2.4683), will download and install from remote
Successfully Installed graviostudio
```

Installation takes a few fields:

```cmd
Usage: graviomanager.exe install [OPTIONS] <NAME> [BUILD_OR_BRANCH]

Arguments:
  <NAME>
  [BUILD_OR_BRANCH]

Options:
  -f, --flavor <FLAVOR>
          Product flavor (e.g.,, Sideloading, Arm64 etc)
  -a, --automatic-upgrade <AUTOMATIC_UPGRADE>
          Whether to find newer build versions, if `build` isnt specified. Leave empty to be prompted. [possible values: true, false]
  -h, --help
          Print help
  -V, --version
          Print version
```

Build Or Branch takes either a specific version (e.g., `5.2.1.7333`), or a
branch/tag, (e.g., `develop`, `test_oauth`, etc). If given a branch, the most
recent successful build will be installed.

# Configuration

The data gman works with comes from the `gman_config_client.json5`. This file is
searched for in the following order:

- As specified by the `--config-path` argument
- Current working directory of the process / shell
- Next to the gman executable file

If the file is not found, you can run the following commands to generate a new
one in your current working directory:

```bash
gman.exe config --sample
```
