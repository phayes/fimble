# fimble

WORK IN PROGRESS

Simple command-line [File Integrity Monitoring](https://en.wikipedia.org/wiki/File_integrity_monitoring)

Fimble lets you ensure the integrity of key files and directories to ensure that they haven't changed. It does this by computing a cryptographically secure digest of a known good state, then comparing the current state against this known good state. 

Fimble aims to be **very fast**, and can do a file-integrity check of the source code of the linux kernel in about two second with a warm disk cache. Despite being very fast, it doesn't take shortcuts and fully hashes all files and file attributes every time.

## Usage

```bash

# Simply compute the digest of a directory
fimble hash /var/my/very/sensitive/files

# Create a manifest of a known good state
fimble build-manifest /var/my/very/sensitive/files > known_good.fimble_manifest

# View the manifest details
fimble view-manifest known_good.fimble_manifest

# Some time passes, possibly bad things happen...

# Check the current system against the manifest to ensure nothing has changed. This is very fast.
fimble check-manifest known_good.fimble_manifest
```

## How it works

Fimble works by computing the [blake3](https://github.com/BLAKE3-team/BLAKE3) cryptographic hash of the specified directories. This is very fast and is done with very little overhead. 

To create a manifest, fimble creates a space-efficient bloom-filter. This is fairly expensive, but the resulting manifest is small and easy to pass around or check into version control.

To check the current status of a system, fimble takes a two step process:
  1. First fimble does a quick-check, computing the blake3 digest of the system and checking this against the master digest in the manifest. If no digest mismatch is found, then we know the system is unaltered and we are done.
  2. If there is a digest mismatch, fimble does an in-depth analysis to see where the differences are.
  
## Gotchas and solutions

1. If the manifest file is too large for your liking, you don't need to use it. Instead just run `fimble hash /my/path` and check the resulting digest against a known good digest. The downside is that you will need to manually determine what has changed if there is a digest mismatch.

2. Fimble doesn't detect internal changes to serial or block devices, although it does detect additions, removals and permission changes for devices.
