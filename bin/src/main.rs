use clap::{App, Arg, SubCommand};

pub fn main() {
    let app = App::new("fimble")
        .version("1.0")
        .about("File Integrity Monitoring")
        .author("Patrick Hayes <patrick.d.hayes@gmail.com>")
        .subcommand(
            SubCommand::with_name("check")
                .about("Computes digest for directories")
                .arg(
                    Arg::with_name("PATHS")
                        .help("Directories or files to check")
                        .required(true)
                        .multiple(true)
                        .index(1),
                ),
        );

    let matches = app.get_matches();

    if let Some(ref matches) = matches.subcommand_matches("check") {
        let paths: Vec<&str> = matches.values_of("PATHS").unwrap().collect();

        let scanner = fimble::Scanner::new();
        let digest = scanner.scan(paths[0]).unwrap();

        dbg!(digest);
    }
}
