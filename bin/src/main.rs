use clap::{App, Arg, SubCommand};
use libflate::gzip::{Decoder, Encoder};
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
        let digest = scanner.scan(&path).unwrap_or_else(|e| {
            eprintln!("Error scanning directory: {}", e);
            std::process::exit(1)
        });

        println!("{}", digest.to_hex());
    }

    // build-manifest
    if let Some(matches) = matches.subcommand_matches("build-manifest") {
        let path = matches.value_of("PATH").unwrap();

        let scanner = fimble::Scanner::new();
        let manifest = scanner.build_manifest(&path).unwrap_or_else(|e| {
            eprintln!("Error scanning directory: {}", e);
            std::process::exit(1)
        });

        let serialized = rmp_serde::to_vec(&manifest).expect("Error encoding manifest file");

        let mut buffer = Vec::new();
        let mut gzip_encoder = Encoder::new(&mut buffer).unwrap();
        gzip_encoder.write_all(&serialized).unwrap();
        let encoded = gzip_encoder.finish().into_result().unwrap();

        io::stdout().write_all(&encoded).unwrap_or_else(|e| {
            eprintln!("Error writing manifest file: {}", e);
            std::process::exit(1)
        });
    }

    // view-manifest
    if let Some(matches) = matches.subcommand_matches("view-manifest") {
        let path = matches.value_of("MANIFEST-PATH").unwrap();

        let mut f = File::open(path).unwrap_or_else(|e| {
            eprintln!("Error opening manifest file: {}", e);
            std::process::exit(1)
        });

        let mut gzip_decoder = Decoder::new(&mut f).unwrap();
        let mut buffer = Vec::new();
        gzip_decoder.read_to_end(&mut buffer).unwrap_or_else(|e| {
            eprintln!("Error reading manifest file: {}", e);
            std::process::exit(1)
        });

        let manifest: fimble::Manifest = rmp_serde::from_read_ref(&buffer).unwrap_or_else(|e| {
            eprintln!("Error reading manifest file: {}", e);
            std::process::exit(1)
        });

        println!("path:   {}", &manifest.path);
        println!("digest: {}", hex::encode(&manifest.digest));
        println!("files:");

        // Print files in sort order
        let mut files = Vec::with_capacity(manifest.files.len());
        for (k, v) in manifest.files.into_iter() {
            files.push((k, v));
        }
        files.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (k, v) in files.into_iter() {
            println!("    {}: {}", hex::encode(v), k.to_string_lossy());
        }
    }

    // check-manifest
    if let Some(matches) = matches.subcommand_matches("check-manifest") {
        let path = matches.value_of("MANIFEST-PATH").unwrap();

        let mut f = File::open(path).unwrap_or_else(|e| {
            eprintln!("Error opening manifest file: {}", e);
            std::process::exit(1)
        });
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).unwrap_or_else(|e| {
            eprintln!("Error reading manifest file: {}", e);
            std::process::exit(1)
        });

        let manifest: fimble::Manifest = rmp_serde::from_read_ref(&buffer).unwrap_or_else(|e| {
            eprintln!("Error reading manifest file: {}", e);
            std::process::exit(1)
        });

        match manifest.quick_check() {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("{}", e);
                match manifest.scan_check() {
                    Ok(changed) => {
                        if changed.len() != 0 {
                            for file in changed.into_iter() {
                                println!("{}", file.to_string_lossy());
                            }
                            std::process::exit(1)
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1)
                    }
                }
            }
        }
    }
}
