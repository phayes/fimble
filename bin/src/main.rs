use clap::{App, Arg, SubCommand};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::Write;

pub fn main() {
    let app = App::new("fimble")
        .version("1.0")
        .about("File Integrity Monitoring")
        .author("Patrick Hayes <patrick.d.hayes@gmail.com>")
        .subcommand(
            SubCommand::with_name("hash")
                .about("Computes the digest for a directory")
                .arg(
                    Arg::with_name("PATH")
                        .help("Directories or files to check")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("build-manifest")
                .about("Build a manifest for a directory")
                .arg(
                    Arg::with_name("PATH")
                        .help("Directories or files to check")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("view-manifest")
                .about("View metadata for a manifest")
                .arg(
                    Arg::with_name("MANIFEST-PATH")
                        .help("path to the manifest file")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("check-manifest")
                .about("Check if a manifest is valid against the local filesystem")
                .arg(
                    Arg::with_name("MANIFEST-PATH")
                        .help("path to the manifest file")
                        .required(true)
                        .index(1),
                ),
        );

    let matches = app.get_matches();

    // hash
    if let Some(matches) = matches.subcommand_matches("hash") {
        let path = matches.value_of("PATH").unwrap();

        let scanner = fimble::Scanner::new();
        let digest = scanner.scan(&path).unwrap(); // TODO unwrap

        println!("{}", digest.to_hex());
    }

    // build-manifest
    if let Some(matches) = matches.subcommand_matches("build-manifest") {
        let path = matches.value_of("PATH").unwrap();

        let scanner = fimble::Scanner::new();
        let manifest = scanner.build_manifest(&path).unwrap(); // TODO unwrap

        let encoded = rmp_serde::to_vec(&manifest).unwrap(); // TODO unwrap

        io::stdout().write_all(&encoded).unwrap(); // TODO unwrap
    }

    // view-manifest
    if let Some(matches) = matches.subcommand_matches("view-manifest") {
        let path = matches.value_of("MANIFEST-PATH").unwrap();

        let mut f = File::open(path).unwrap(); // TODO unwrap
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).unwrap(); // TOOD unwrap

        let manifest: fimble::Manifest = rmp_serde::from_read_ref(&buffer).unwrap();

        println!("path:   {}", &manifest.path);
        println!("digest: {}", hex::encode(&manifest.digest));
    }

    // check-manifest
    if let Some(matches) = matches.subcommand_matches("check-manifest") {
        let path = matches.value_of("MANIFEST-PATH").unwrap();

        let mut f = File::open(path).unwrap(); // TODO unwrap
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).unwrap(); // TOOD unwrap

        let manifest: fimble::Manifest = rmp_serde::from_read_ref(&buffer).unwrap();

        match manifest.quick_check() {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                println!("{}", e);
                match manifest.scan_check() {
                    Ok(_) => unreachable!(),
                    Err(e) => {
                        println!("{}", e);
                        std::process::exit(1)
                    }
                }
            }
        }
    }
}
