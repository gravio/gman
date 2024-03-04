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

# SBOM and Checksum

a Software Bill-of-Materials is generated after each build, located at
`sbom.sbdx`. A checksum of the build artifacts is also produced, locatedt at
`release.hash`

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

- As specified by a leading `config-path` argument if supplied,
- Current working directory of the process / shell (`./`)
- Directory the gman executable is located in
- Every parent directory of the executable, popped one by one until the root of
  the filesystem

If the file is not found, you can run the following commands to generate a new
one in your current working directory:

```bash
gman.exe config --sample
```

<details>
<summary>Configuration example file</summary>

`gman_client_config.json5`

```json5
{
    /* 
        Log levels to allow for higher diagnostics printing to console
       Allowed values include: 
        - Off
        - Trace
        - Debug
        - Info
        - Warn
        - Error
    */
  "LogLevel": "OFF",

  
    /*
        Repositories to search for installation cadidates and updates

        Credentials can either be a BearerToken (access token), acquired via the TeamCity webpanel for your user under Profile,
        or your Username/Password
    */
  "Repositories": [
    {
      "Name": "SampleRepository", // User defined name of the repository
      "RepositoryType": "TeamCity", // Type of repository fyi:(nf, 3/2/24): Only TeamCity is supported 
    // Platform for Binary artifacts found on the repository. Valid platform values are { Windows, Mac, }
      "Platforms": [
        "Windows",
        "Mac"
      ],
      "RepositoryServer": "yourbuildserver.yourcompany.example.com", // address of the server
      "RepositoryCredentials": {
        "Type": "BearerToken", // either `BearerToken` or `BasicAuth`
        "Token": "your_token" // API key from TeamCity
      },
      "Products": [
        "SampleProduct" // Products that this repository handles, defined by the `Products` array lower downs
      ]
    }
  ],
  // Mostly just for windows, used to match AppX, MSI, and MSIX installer identities
  "PublisherIdentities": [
    {
      "Name": "SomeCompany Windows Identifier", // Display name for the publisher
      "Id": "CN=ab94ddc1-6575-33ed-8832-1a5d98a25117", // String that will be matched against inside the binary metadata
      "Platforms": [
        // Platforms this identity is valid for
        "Windows"
      ],
      "Products": [
        // (Optional) products this identity is valid for
        "SomeProduct"
      ]
    }
  ],
  // Array of actual products found on the build servers. This is what is listed, installed and uninstalled
  "Products": [
    {
      "Name": "SampleProduct", // User defined name of the product. This will appear in the printed CLI output
      // One product can have multiple different flavors of actual binary artifact, such as for Sideloading, or Docker, or Mac/Windows versions
      "Flavors": [
        {
          "Platform": "Windows", // Target platform of the binary
          "Id": "UWP", // User defined Id of this flavor
          "TeamCityMetadata": {
            "TeamCityId": "SomeUwpSample", // TeamCity project id
            "TeamCityBinaryPath": "path/to/WindowsUWP.zip" // Path on TeamCity to the final artifact
          },
          "PackageType": "AppX", // Type of Package. Valid values are one of { Msi, MsiX, AppX, App, Dmg, Pkg, Apk, Ipa }
          // Flavor-specific metadata used for matching products on the users machine
          "Metadata": {
            // for UWP (Appx) binaries, this is the name of the product as known to Microsoft
            "NameRegex": "some.uwp.sampleproduct"
          },
          // If true, will attempt to launch the application automoatically after installation
          "Autorun": false
        },
        {
          "Platform": "Mac",
          "Id": "MacApp",
          "TeamCityMetadata": {
            "TeamCityId": "SomeMacSample",
            "TeamCityBinaryPath": "path/to/MacApp.dmg"
          },
          "PackageType": "App",
          "Metadata": {
            // The Id of the publisher/app as known to Apple
            "CFBundleIdentifier": "com.somecompany.sampleproduct",
            // The name of the app as known to Apple
            "CFBundleName": "SampleProduct"
          },
          "Autorun": false
        }
      ]
    }
  ]
}
```

</details>

### PackageType

| Package Type | Platform | Description                          |
| ------------ | -------- | ------------------------------------ |
| Msi          | Windows  | Traditional Microsoft .msi installer |
| MsiX         | Windows  | Modern Microsoft installer           |
| AppX         | Windows  | Windows UWP package type             |
| App          | macOS    | Mac .App package type                |
| Pkg          | macOS    | Mac .pkg package type                |
| Apk          | Android  | Android apk package type             |
| Ipa          | iOS      | iOS app package type                 |

### Platform

| Platform     | String  |
| ------------ | ------- |
| Windows      | Windows |
| Mac          | macOS   |
| Linux        | Linux   |
| Android      | Android |
| iOS          | iOS     |
| Docker       | Docker  |
| Raspberry Pi | rpi     |
