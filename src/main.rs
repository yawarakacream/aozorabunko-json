pub mod accent_composer;
pub mod book_file_parser;
pub mod jis_x_0213;
pub mod parser;
pub mod utility;

use anyhow::{bail, ensure, Context, Result};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use parser::parse_index_list_extended;
use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    path::PathBuf,
};

use crate::{
    book_file_parser::{parse_ruby_txt, tokenize_ruby_txt},
    utility::{Date, ZipReader},
};

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
    ensure!(
        aozorabunko_path.exists(),
        "File not found: {}",
        aozorabunko_path.display()
    );

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
        let csv_zip_path = aozorabunko_path.join("index_pages/list_person_all_extended_utf8.zip");
        let csv_zip_file = File::open(csv_zip_path).unwrap();
        let mut csv_zip_reader = ZipReader::new(csv_zip_file)?;

        let mut csv_entry = csv_zip_reader.get_by_path("list_person_all_extended_utf8.csv")?;
        let csv_data = csv_entry.as_string()?;

        parse_index_list_extended(&csv_data)?
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

    let book_root_path = &output_path.join("book");

    println!("Processing cards...");

    // 人物著作権 が あり の著者の ID
    let author_ids_with_copyright: HashSet<_> = aozorabunko_index_list
        .authors
        .iter()
        .filter(|&a| a.copyright)
        .map(|a| a.id)
        .collect();

    let pb = create_progress_bar(aozorabunko_index_list.books.len() as u64);
    for book in aozorabunko_index_list.books.iter().progress_with(pb) {
        let book_directory_path = book_root_path.join(book.id.to_string());
        fs::create_dir_all(&book_directory_path).unwrap();

        let author_ids: Vec<usize> = aozorabunko_index_list
            .book_authors
            .iter()
            .filter(|&ba| &ba.book_id == &book.id)
            .map(|ba| ba.author_id)
            .collect();

        // 著作権確認
        if book.copyright
            || author_ids
                .iter()
                .any(|aid| author_ids_with_copyright.contains(aid))
        {
            continue;
        }

        // .txt
        if let Some(txt_url) = &book.txt_url {
            if !txt_url.starts_with("https://www.aozora.gr.jp/") {
                continue;
            }

            (|| {
                ensure!(&txt_url.ends_with("zip"), "Not zip file");

                let txt_zip_path =
                    aozorabunko_path.join(&txt_url["https://www.aozora.gr.jp/".len()..]);
                let txt_zip_file = File::open(&txt_zip_path).unwrap();
                let mut txt_zip_reader = ZipReader::new(txt_zip_file)?;

                let mut txt_bytes = None;
                for i in 0..txt_zip_reader.len() {
                    let mut entry = txt_zip_reader.get_by_index(i).unwrap();
                    if !entry.name().to_lowercase().ends_with(".txt") {
                        continue;
                    }

                    ensure!(txt_bytes.is_none(), ".txt file exists more than 1");

                    txt_bytes = Some(entry.as_bytes()?);
                }

                let txt_bytes = txt_bytes.context(".txt file is not found")?;
                let txt = encoding_rs::SHIFT_JIS.decode(&txt_bytes).0.into_owned();

                if txt_url.contains("ruby") {
                    // 2010 年 4 月 1 日に公布されたフォーマットに従うパース
                    static VALID_DATE: Date = Date::YMD {
                        year: 2010,
                        month: 4,
                        date: 1,
                    };

                    let content = tokenize_ruby_txt(&txt).and_then(|x| parse_ruby_txt(&x));

                    match content {
                        Ok(content) => {
                            fs::write(
                                &book_directory_path.join("content_from_ruby-txt.json"),
                                serde_json::to_string_pretty(&content)?,
                            )?;
                        }
                        Err(err) => {
                            if book.published_at.is_equivalent_or_later(&VALID_DATE)
                                && book.updated_at.is_equivalent_or_later(&VALID_DATE)
                            {
                                return Err(err);
                            }
                            println!(
                                "[WARN] Failed to process book ruby-txt and ignored: {}, 「{}」",
                                book.id, book.title
                            );
                        }
                    }
                }

                Ok(())
            })()
            .with_context(|| format!("Failed to process book zip: {:?}", &book))?;
        }
    }

    println!("Finished.");

    Ok(())
}

fn create_progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template(
            "{percent:>3}% [{wide_bar:.cyan/blue}] {pos}/{len} [{elapsed_precise} < {eta_precise}]",
        )
        .unwrap()
        .progress_chars("#-"),
    );
    pb
}
