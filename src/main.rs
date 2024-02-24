pub mod parser;
pub mod utility;

use anyhow::{bail, Context, Result};
use parser::parse_index_list_extended;
use std::{
    env,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};
use zip::ZipArchive;

struct Args {
    aozorabunko_path: String,
    output_path: String,
}

fn get_args() -> Result<Args> {
    let args: Vec<String> = env::args().skip(1).collect();

    let opts = getopts::Options::new();

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => bail!(f),
    };

    if matches.free.len() != 2 {
        bail!("path to aozorabunko repository and output are required.")
    }

    let aozorabunko_path = matches.free[0].clone();
    let output_path = matches.free[1].clone();

    Ok(Args {
        aozorabunko_path,
        output_path,
    })
}

fn main() -> Result<()> {
    let args = get_args()?;

    let aozorabunko_path = PathBuf::from(&args.aozorabunko_path);
    let output_path = PathBuf::from(&args.output_path);

    // create output directory
    // it fails if output directory already exists, expect for "./build"
    if output_path.exists() {
        if output_path == PathBuf::from("./build") {
            fs::remove_dir_all(&output_path).unwrap();
        } else {
            bail!("Already exists: {}", &args.output_path);
        }
    }
    fs::create_dir(&output_path)
        .with_context(|| format!("Failed to create output directory: {}", &args.output_path))?;

    println!("Processing list_person_all_extended...");

    let aozorabunko_index_list = {
        let input_path = aozorabunko_path.join("index_pages/list_person_all_extended_utf8.zip");

        let zip_file =
            File::open(input_path).context("Failed to open list_person_all_extended_utf8.zip")?;

        let mut zip_archive = ZipArchive::new(zip_file)
            .context("Failed to read list_person_all_extended_utf8.zip")?;

        let mut csv = zip_archive
            .by_name("list_person_all_extended_utf8.csv")
            .context("Failed to open list_person_all_extended_utf8.csv")?;

        let mut list_person_all_extended_csv = String::new();
        csv.read_to_string(&mut list_person_all_extended_csv)
            .context("Failed to read list_person_all_extended_utf8.csv")?;

        parse_index_list_extended(&list_person_all_extended_csv)?
    };

    fs::write(
        &output_path.join("books.json"),
        serde_json::to_string(&aozorabunko_index_list.books)?,
    )?;

    fs::write(
        &output_path.join("authors.json"),
        serde_json::to_string(&aozorabunko_index_list.authors)?,
    )?;

    fs::write(
        &output_path.join("book_authors.json"),
        serde_json::to_string(&aozorabunko_index_list.book_authors)?,
    )?;

    println!("Finished.");

    Ok(())
}
