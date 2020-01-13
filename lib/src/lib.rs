#[macro_use]
extern crate failure;

use blake3::{Hash, Hasher};
use growable_bloom_filter::GrowableBloom;
use jwalk::{DirEntry, WalkDir};
use os_str_bytes::OsStrBytes;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::unix::fs::Permissions;

#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;

#[cfg(not(windows))]
use std::os::unix::fs::FileTypeExt;

mod filehash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub path: String,
    pub digest: [u8; 32],
    pub bloom: GrowableBloom,
}

impl Manifest {
    pub fn check_hash(&self, digest: &Hash) -> Result<(), Error> {
        let check_hash: Hash = self.digest.into();
        match check_hash.eq(digest) {
            true => Ok(()),
            false => Err(Error::ManifestCheckFail),
        }
    }

    pub fn quick_check(&self) -> Result<(), Error> {
        let walker = WalkDir::new(&self.path).sort(true).preload_metadata(true);
        let mut hasher = blake3::Hasher::new();
        for entry in walker {
            let entry: jwalk::DirEntry = entry?;
            hash_entry(entry, &mut hasher)?;
        }

        let digest = hasher.finalize();
        self.check_hash(&digest)
    }

    pub fn scan_check(&self) -> Result<(), Error> {
        let walker = WalkDir::new(&self.path).sort(true).preload_metadata(true);
        let mut hasher = blake3::Hasher::new();
        for entry in walker {
            let entry: jwalk::DirEntry = entry?;
            let filename = &entry.file_name.to_string_lossy().into_owned();

            hash_entry(entry, &mut hasher)?;

            let checkpoint = hasher.clone().finalize();

            if !self.bloom.contains(&checkpoint) {
                dbg!("Error on file: {}", filename);
                return Err(Error::ManifestCheckFail);
            }
        }

        let digest = hasher.finalize();

        if !&digest.eq(&self.digest) {}

        Ok(())
    }
}

pub struct Scanner {}

impl Scanner {
    pub fn new() -> Self {
        Scanner {}
    }

    pub fn scan<P: AsRef<Path>>(&self, root: P) -> Result<Hash, Error> {
        let walker = WalkDir::new(root).sort(true).preload_metadata(true);
        let mut hasher = blake3::Hasher::new();
        for entry in walker {
            let entry: jwalk::DirEntry = entry?;
            hash_entry(entry, &mut hasher)?;
        }

        Ok(hasher.finalize())
    }

    pub fn scan_multiple<P: AsRef<Path>>(&self, roots: &[P]) -> Result<Hash, Error> {
        let mut hasher = blake3::Hasher::new();
        for path in roots {
            let digest = self.scan(path)?;
            hasher.update(digest.as_bytes());
        }

        Ok(hasher.finalize())
    }

    pub fn build_manifest<P: AsRef<Path>>(&self, root: P) -> Result<Manifest, Error> {
        let walker = WalkDir::new(root.as_ref())
            .sort(true)
            .preload_metadata(true);
        let mut hasher = blake3::Hasher::new();
        let mut checkpoints = Vec::new();
        for entry in walker {
            let entry: jwalk::DirEntry = entry?;
            hash_entry(entry, &mut hasher)?;

            checkpoints.push(hasher.clone().finalize());
        }

        let mut bloom = GrowableBloom::new(0.001, checkpoints.len());
        for checkpoint in checkpoints {
            bloom.insert(checkpoint);
        }

        let digest = hasher.finalize();

        Ok(Manifest {
            path: format!("{}", root.as_ref().display()),
            digest: digest.as_bytes().to_owned(),
            bloom,
        })
    }
}

// TODO: filename in error
fn hash_entry(entry: DirEntry, hasher: &mut Hasher) -> Result<(), Error> {
    // Destructure the the entry
    let DirEntry {
        depth: _,
        file_type: _,
        content_spec: _,
        parent_spec,
        content_error: _,
        metadata,
        file_name,
    } = entry;

    // Construct the full path to the entry
    let path: PathBuf = parent_spec.path.join(&file_name);

    // Unwrap the metadata
    let metadata = metadata
        .expect("Cannot fetch file metadata")
        .map_err(|e| Error::EntryErr(e, format!("{}", &path.display())))?;

    // Update the hash as per the filetype
    let filetype = metadata.file_type();
    if filetype.is_dir() {
        hasher.update(&[0]);
    } else if filetype.is_symlink() {
        hasher.update(&[1]);
    } else if filetype.is_file() {
        hasher.update(&[2]);
    } else {
        // It's something else we don't know about
        #[cfg(windows)]
        hasher.update(&[255]);

        // If we're not on windows hash the device type
        #[cfg(not(windows))]
        {
            if filetype.is_block_device() {
                hasher.update(&[3]);
            } else if filetype.is_char_device() {
                hasher.update(&[4]);
            } else if filetype.is_fifo() {
                hasher.update(&[5]);
            } else if filetype.is_socket() {
                hasher.update(&[6]);
            } else {
                // It's something else we don't know about
                hasher.update(&[255]);
            }
        }
    }

    // Update the hash as per file permissions
    let permissions = metadata.permissions();
    if permissions.readonly() {
        hasher.update(&[0]);
    } else {
        hasher.update(&[1]);
    }

    // If we're not on windows looks an the entire mode permissions
    #[cfg(not(windows))]
    {
        use std::mem::transmute;
        let permissions = metadata.permissions();
        let bytes: [u8; 4] = unsafe { transmute(permissions.mode().to_be()) };
        hasher.update(&bytes);
    }

    // If it's a file, hash it's contents
    if filetype.is_file() {
        filehash::hash_file(hasher, &path, &metadata)?;
    }

    // If it's a symlink, hash it's target
    if filetype.is_symlink() {
        let link = std::fs::read_link(&path)
            .map_err(|e| Error::EntryErr(e, format!("{}", &path.display())))?;
        hasher.update(link.as_os_str().to_bytes().as_ref());
    }

    // Finally update the hash with the filename
    let os_path = path.into_os_string();
    hasher.update(&os_path.to_bytes());

    Ok(())
}

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "<unknown path>: io error: {}", 0)]
    IoErr(std::io::Error),

    #[fail(display = "{}: io error: {}", 1, 0)]
    EntryErr(std::io::Error, String),

    #[fail(display = "Manifest file integrity check failed - something has changed")]
    ManifestCheckFail,
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IoErr(error)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn basic_test() {
        let scanner = Scanner::new();
        let manifest = scanner.build_manifest("./test").unwrap();

        let hash = scanner.scan("./test").unwrap();

        manifest.check_hash(&hash).unwrap();
        manifest.quick_check().unwrap();
        manifest.scan_check().unwrap();
    }

    #[test]
    fn failure_test() {
        // Copy `test` to a temporary directory that we can modify
        let temp_dir = "./temp_test";

        copy_dir::copy_dir("./test", temp_dir).unwrap();

        //  Make sure we clean up
        let _guard = scopeguard::guard((), |_| fs_extra::dir::remove(temp_dir).unwrap());

        // Create the scanner and the manifest
        let scanner = Scanner::new();
        let manifest = scanner.build_manifest(temp_dir).unwrap();

        // So far so good - nothig has changed
        let hash = scanner.scan(temp_dir).unwrap();
        manifest.check_hash(&hash).unwrap();
        manifest.quick_check().unwrap();
        manifest.scan_check().unwrap();

        // Modify a file
        fs_extra::file::write_all("./temp_test/README.md", "   EXTRA DATA").unwrap();
        let hash = scanner.scan(temp_dir).unwrap();

        // Assert that both the quick_check and the full scan-check fails
        assert!(manifest.check_hash(&hash).is_err());
        assert!(manifest.quick_check().is_err());
        assert!(manifest.scan_check().is_err());
    }
}
