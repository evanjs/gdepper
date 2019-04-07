#[macro_use]
extern crate clap;
use clap::App;
#[macro_use]
extern crate log;
use env_logger;
use std::process::Command;
extern crate tar;
use std::io::{BufReader, Read, BufRead};
use tar::{Builder, Header};
extern crate flate2;
use flate2::Compression;
use flate2::write::GzEncoder;
extern crate failure;

use std::path::PathBuf;
extern crate pathdiff;

extern crate zstd;
use zstd::stream::write::*;
use zstd::stream::*;

fn main() -> Result<(), failure::Error> {
    env_logger::init();
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let pretend = matches.occurrences_of("dry_run");
    let package = value_t_or_exit!(matches, "package", String);
    let destination = value_t!(matches, "destination", String).unwrap_or("/tmp".to_owned());
    let format = value_t!(matches, "format", String).unwrap_or("xz".to_owned());
    let filters = value_t!(matches, "filters", String)
        .unwrap_or("obj,sym,dev,conf,cmd,doc,man,info".to_owned());

    let mut package_file_query_command = Command::new("equery");
    package_file_query_command
        .arg("-C")
        .arg("-q")
        .arg("f")
        .arg(format!("--filter={}", &filters))
        .arg(&package);

    let package_files = package_file_query_command
        .output()
        .expect(format!("Failed to query package files for {}", &package).as_str());

    let mut files = Vec::<String>::new();

    let reader = BufReader::new(&*package_files.stdout);

    reader.lines().map(|l|l.unwrap()).for_each(|i: String| {
        let path = std::path::PathBuf::from(i);
        files.push(path.to_str().unwrap().to_string());
    });

    let mut destination = std::path::Path::new(destination.as_str()).join(package);
    //destination.set_extension("tar.gz");
    destination.set_extension("tar.zstd");
    trace!("{:#?}", &destination);
    let archive = std::fs::File::create(destination).expect("Failed to create output file");


    //let enc = GzEncoder::new(archive, Compression::default());
    let enc = Encoder::new(archive, zstd::DEFAULT_COMPRESSION_LEVEL).unwrap().auto_finish();
    let mut builder = Builder::new(enc);

    let cur_dir = std::env::current_dir()?;
    nix::unistd::chdir("/");

    files.iter().for_each(|f| {
        let this_path: PathBuf = f.into();
        let stripped = this_path.strip_prefix("/").unwrap();
        builder.append_path(&stripped).expect("Failed to add path to archive");

    });

    //builder.into_inner()?.finish()?;
    builder.into_inner()?;

    Ok(())
}
