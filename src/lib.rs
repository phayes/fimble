#[macro_use]
extern crate failure;

use blake3::Hash;
use os_str_bytes::OsStrBytes;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use walkdir::WalkDir;

#[cfg(windows)]
use std::os::unix::fs::Permissions;

#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;

#[cfg(not(windows))]
use std::os::unix::fs::FileTypeExt;

pub struct Config {}

pub struct Scanner {
    pub config: Config,
}

impl Scanner {
    pub fn new() -> Self {
        Scanner { config: Config {} }
    }

    pub fn scan<P: AsRef<Path>>(&self, root: P) -> Result<Hash, Error> {
        let walker = WalkDir::new(root);
        let mut hasher = blake3::Hasher::new();
        for entry in walker {
            let entry: walkdir::DirEntry = entry?;

            // Update the hash as per the filename
            hasher.update(entry.path().as_os_str().to_bytes().as_ref());

            // Update the hash as per the filetype
            let filetype = entry.file_type();
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
            let metadata = entry.metadata()?;
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
            if filetype.is_file() || filetype.is_symlink() {
                let file = File::open(entry.path())?;
                let mut buf_reader = BufReader::new(file);

                let buffer = buf_reader.fill_buf()?;
                hasher.update(&buffer);
                let len = buffer.len();
                buf_reader.consume(len);
            }
        }

        Ok(hasher.finalize())
    }
}

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "walkdir error: {}", 0)]
    WalkDirError(walkdir::Error),

    #[fail(display = "io error: {}", 0)]
    IOError(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IOError(error)
    }
}

impl From<walkdir::Error> for Error {
    fn from(error: walkdir::Error) -> Self {
        Error::WalkDirError(error)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn it_works() {
        let scanner = Scanner::new();
        let hash = scanner.scan(".").unwrap();

        dbg!(hash);
    }
}
