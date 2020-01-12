#[macro_use]
extern crate failure;

use blake3::{Hash, Hasher};
use growable_bloom_filter::GrowableBloom;
use jwalk::{DirEntry, WalkDir};
use os_str_bytes::OsStrBytes;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::unix::fs::Permissions;

#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;

#[cfg(not(windows))]
use std::os::unix::fs::FileTypeExt;

pub struct Scanner {}

pub struct Manifest {
    pub digest: Hash,
    pub bloom: GrowableBloom,
}

impl Scanner {
    pub fn new() -> Self {
        Scanner {}
    }

    pub fn scan<P: AsRef<Path>>(&self, root: P) -> Result<Hash, Error> {
        let walker = WalkDir::new(root).sort(true).preload_metadata(true);
        let mut hasher = blake3::Hasher::new();
        for entry in walker {
            let entry: jwalk::DirEntry = entry?;
            self.hash_entry(entry, &mut hasher)?;
        }

        Ok(hasher.finalize())
    }

    pub fn build_manifest<P: AsRef<Path>>(&self, root: P) -> Result<Manifest, Error> {
        let walker = WalkDir::new(root).sort(true).preload_metadata(true);
        let mut hasher = blake3::Hasher::new();
        let mut checkpoints = Vec::new();
        for entry in walker {
            let entry: jwalk::DirEntry = entry?;
            self.hash_entry(entry, &mut hasher)?;

            checkpoints.push(hasher.clone().finalize());
        }

        let mut bloom = GrowableBloom::new(0.00001, checkpoints.len());
        for checkpoint in checkpoints {
            bloom.insert(checkpoint);
        }

        let digest = hasher.finalize();

        Ok(Manifest { digest, bloom })
    }

    pub fn check_manifest<P: AsRef<Path>>(
        &self,
        root: P,
        manifest: &Manifest,
    ) -> Result<(), Error> {
        let walker = WalkDir::new(root).sort(true).preload_metadata(true);
        let mut hasher = blake3::Hasher::new();
        for entry in walker {
            let entry: jwalk::DirEntry = entry?;
            let filename = &entry.file_name.to_string_lossy().into_owned();

            self.hash_entry(entry, &mut hasher)?;

            let checkpoint = hasher.clone().finalize();

            if !manifest.bloom.contains(&checkpoint) {
                dbg!("Error on file: {}", filename);
            }
        }

        let digest = hasher.finalize();

        if !&digest.eq(&manifest.digest) {}

        Ok(())
    }

    // TODO: filename in error
    fn hash_entry(&self, entry: DirEntry, hasher: &mut Hasher) -> Result<(), Error> {
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
                    // TODO: return error??
                    hasher.update(&[255]);
                }
            }

            // It's something else we don't know about
            // TODO: return error??
            #[cfg(windows)]
            hasher.update(&[255]);
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

        // If it's a file or a symlink, hash it's contents
        if filetype.is_file() {
            let file = File::open(&path)
                .map_err(|e| Error::EntryErr(e, format!("{}", &path.display())))?;
            let mut buf_reader = BufReader::new(file);

            let buffer = buf_reader
                .fill_buf()
                .map_err(|e| Error::EntryErr(e, format!("{}", &path.display())))?;
            hasher.update(&buffer);
            let len = buffer.len();
            buf_reader.consume(len);
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
}

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "<unknown path>: io error: {}", 0)]
    IoErr(std::io::Error),

    #[fail(display = "{}: io error: {}", 1, 0)]
    EntryErr(std::io::Error, String),
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
    fn it_works() {
        let scanner = Scanner::new();
        let hash = scanner.scan("/opt/opticalgym").unwrap();

        dbg!(hash);
    }
}
