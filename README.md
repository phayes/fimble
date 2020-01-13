# fimble

WORK IN PROGRESS

Simple command-line [File Integrity Monitoring](https://en.wikipedia.org/wiki/File_integrity_monitoring)

Fimble lets you ensure the integrity of key files and directories to ensure that they haven't changed. It does this by computing a cryptographically secure digest of a known good state, then comparing the current state against this known good state. 

Fimble aims to be very fast, and can do a file-integrity check of the source code of the linux kernel in about a second with warm caches.

## Usage

```bash

# Simply compute the digest of a directory
fimble hash /var/my/very/sensitive/files

# Create a manifest of a known good state
fimble build-manifest /var/my/very/sensitive/files > known_good.fimble_manifest

# View the manifest details
fimble view-manifest known_good.fimble_manifest

# Some time passes, possibly bad things happen...

# Check the current system against the manifest to ensure nothing has changed
fimble check-manifest known_good.fimble_manifest
```

## How it works

Fimble works by computing the [blake3](https://github.com/BLAKE3-team/BLAKE3) cryptographic hash of the specified directories. This is very fast and is done with very little overhead. 

To create a manifest, fimble creates a space-efficient bloom-filter. This is fairly expensive, but the resulting manifest is small and easy to pass around or check into version control.

To check the current status of a system, fimble takes a two step process:
  1. First fimble does a quick-check, computing the blake3 digest of the system and checking this against the master digest in the manifest.
  2. If no mismatch is found, then we know the system is unaltered and we are done.
  3. If there is a mismatch, fimble utilizes the bloom-filter in the manifest to pinpoint the location of the difference.
  
## Caveats

1. Fimble can only find the "first" instance of a file changing. This limitation is due to using a space-efficient bloom-filter. TODO: Fix this?
2. Fimble can't know if there were changes to serial or block devices.
