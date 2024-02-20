# Gravio Manager

Client: Manages installations of Studio, Hubkit on a (local) machine Server:
Hosts versioned Gravio binaries

# Client

## Example

### Install Studio to the local machine, using the latest develop branch

```bash
gman install gs/win develop
```

## Install Studio to the local machine, using the official version number

```
gman [-h, --help]
    * prints help

gman [-v, --version]
    * prints version

gman [-l, --list] [candidate] [version]
    * shows the list of products available:
        Candidate   | version                   | Identifier
        Hubkit      | develop (5.2.1-7023)      | develop
        Hubkit      | 5.2.0-7010                | 5.2
        gs/win      | develop (5.2.1-8823)      | develop
        gs/mac      | 5.1.0                     | 5.1
        ...

gman [-i, --install] [candidate] [version]
    * installs the version of hubkit/studio requested

gman [-u, --uninstall] [candidate] [version]
    * uninstalls the version of hubkit/studio requested

gman [-p, --provision] [-f, --file]
    * provisions a server according to the input file
```

# Server

```
gman server [{-b, --bind} <address>]  [{-p, --port} <port>] 
gman server [-c, --config] <config>


gman server --config myconfig.json5
```

## Config.json5

```json5
```

# Provisioning

gman can provision an aws server with a setup specified by a `.json5` file:

```json5
{
    "todo"
}
```
