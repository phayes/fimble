use std::fs::File;
use std::fs::Metadata;
use std::io;
use std::io::prelude::*;

// Errors from this function get handled by the file loop and printed per-file.
pub(crate) fn hash_file(
    hasher: &mut blake3::Hasher,
    filepath: &std::path::PathBuf,
    metadata: &Metadata,
) -> Result<(), io::Error> {
    let file = File::open(filepath)?;
    if !maybe_hash_memmap(hasher, &file, metadata)? {
        // The fast-path didn't go, do it via the slow-path
        hash_reader(hasher, file)?;
    }
    Ok(())
}

// The slow path, for inputs that we can't memmap.
fn hash_reader(hasher: &mut blake3::Hasher, mut reader: impl Read) -> Result<(), io::Error> {
    std::io::copy(&mut reader, hasher)?;
    Ok(())
}

fn maybe_memmap_file(file: &File, metadata: &Metadata) -> Result<Option<memmap::Mmap>, io::Error> {
    let file_size = metadata.len();
    Ok(if file_size > isize::max_value() as u64 {
        // Too long to safely map.
        // https://github.com/danburkert/memmap-rs/issues/69
        None
    } else if file_size == 0 {
        // Mapping an empty file currently fails.
        // https://github.com/danburkert/memmap-rs/issues/72
        None
    } else {
        // Explicitly set the length of the memory map, so that filesystem
        // changes can't race to violate the invariants we just checked.
        let map = unsafe {
            memmap::MmapOptions::new()
                .len(file_size as usize)
                .map(&file)?
        };
        Some(map)
    })
}

// The fast path: Try to hash a file by mem-mapping it first. This is faster if
// it works, but it's not always possible.
fn maybe_hash_memmap(
    hasher: &mut blake3::Hasher,
    file: &File,
    metadata: &Metadata,
) -> Result<bool, io::Error> {
    if let Some(map) = maybe_memmap_file(file, metadata)? {
        hasher.update(&map);
        return Ok(true);
    }
    Ok(false)
}
