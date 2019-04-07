#[macro_use]
extern crate clap;
extern crate failure;
extern crate flate2;
#[macro_use]
extern crate log;
extern crate pathdiff;
extern crate smush;
extern crate tar;

use clap::App;
use env_logger;
use flate2::write::GzEncoder;
use flate2::Compression;
use smush::{decode, enabled_encoding, encode, Encoding, Quality};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::Cursor;
use std::io::Write;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::prelude::*;
use std::process::Command;
use tar::Archive;
use tar::{Builder, Header};

fn main() -> Result<(), failure::Error> {
    env_logger::init();
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let pretend = matches.occurrences_of("dry_run");
    let package = value_t_or_exit!(matches, "package", String);
    let destination = value_t!(matches, "destination", String).unwrap_or("/tmp".to_owned());
    let format = value_t!(matches, "format", String).unwrap_or("gz".to_owned());
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

    reader.lines().map(|l| l.unwrap()).for_each(|i: String| {
        let path = std::path::PathBuf::from(i);
        files.push(path.to_str().unwrap().to_string());
    });

    let mut destination = std::path::Path::new(destination.as_str()).join(package);
    let (format_ext, compression) = match format.as_str() {
        "gz" => ("gz", Encoding::Gzip),
        "xz" => ("xz", Encoding::Gzip),
        "zstd" => ("zstd", Encoding::Zstd),
        _ => ("gz", Encoding::Gzip),
    };

    destination.set_extension(format!("tar.{}", format_ext));
    trace!("{:#?}", &destination);
    let file = std::fs::File::create(&destination).expect("Failed to create output file");

    let mut builder = Builder::new(file);

    nix::unistd::chdir("/")?;

    files.iter().for_each(|f| {
        let this_path: PathBuf = f.into();
        let stripped = this_path.strip_prefix("/").unwrap();
        builder
            .append_path(&stripped)
            .expect("Failed to add path to archive");
    });

    builder.finish()?;

    let mut data = builder.into_inner().unwrap();
    data.flush()?;

    let mut file = File::open(&destination)?;
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)?;
    let enc = encode(&mut buffer, compression, Quality::Default)?;

    let mut writer = File::create(&destination)?;
    writer.write(&*enc)?;
    writer.flush()?;

    Ok(())
}
