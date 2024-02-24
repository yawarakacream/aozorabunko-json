use std::collections::{HashMap, HashSet};

use anyhow::{bail, ensure, Context, Result};
use serde::Serialize;

use crate::utility::Date;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    id: String,                  // 人物 ID
    last_name: String,           // 姓
    first_name: String,          // 名
    last_name_kana: String,      // 姓読み
    first_name_kana: String,     // 名読み
    last_name_sort_key: String,  // 姓読みソート用
    first_name_sort_key: String, // 名読みソート用
    last_name_romaji: String,    // 姓ローマ字
    first_name_romaji: String,   // 名ローマ字

    birth_date: String, // 生年月日 (紀元前*世紀 のような表記があり Date は使えない)
    death_date: String, // 没年月日

    copyright: bool, // 人物著作権フラグ
}

#[derive(Debug, PartialEq, Eq, Serialize, Hash)]
#[serde(rename_all = "camelCase")]
pub struct BookAuthor {
    book_id: String,
    author_id: String,
    author_role: String, // 役割フラグ
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalBook {
    title: String,                // 底本名
    publisher_name: String,       // 底本出版社名
    first_edition_date: String,   // 底本初版発行年 (年 とあるが日付が入る)
    input_edition: String,        // 入力に使用した版
    proofreading_edition: String, // 校正に使用した版

    parent_title: String,              // 底本の親本名
    parent_publisher_name: String,     // 底本の親本出版社
    parent_first_edition_date: String, // 底本の親本初版発行年 (年 とあるが日付が入る)
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    id: String,             // 作品 ID
    title: String,          // 作品名
    title_kana: String,     // 作品名読み
    sort_key: String,       // ソート用読み
    subtitle: String,       // 副題
    subtitle_kana: String,  // 副題読み
    original_title: String, // 原題

    writing_system: String, // 文字遣い種別

    copyright: bool, // 作品著作権フラグ

    published_at: Date, // 公開日
    updated_at: Date,   // 最終更新日

    original_book: Vec<OriginalBook>, // 底本

    inputter_name: String,    // 入力者名
    proofreader_name: String, // 校正者名
}

#[derive(Serialize)]
pub struct AozorabunkoIndexList {
    pub authors: Vec<Author>,
    pub books: Vec<Book>,
    pub book_authors: Vec<BookAuthor>,
}

pub fn parse_index_list_extended(
    list_person_all_extended_csv: &str,
) -> Result<AozorabunkoIndexList> {
    let mut reader = csv::Reader::from_reader(list_person_all_extended_csv.as_bytes());

    // let mut records = Vec::<csv::StringRecord>::new();
    // for (i, record) in reader.records().enumerate() {
    //     let record: csv::StringRecord =
    //         record.with_context(|| format!("Failed to parse record at {}", i))?;
    //     records.push(record);
    // }
    // let records = records;

    let mut authors = HashMap::<String, Author>::new();
    let mut books = HashMap::<String, Book>::new();
    let mut book_authors = HashSet::<BookAuthor>::new();

    for (i, record) in reader.records().enumerate() {
        let record: csv::StringRecord =
            record.with_context(|| format!("Failed to parse record at {}", i))?;

        let (author, book, book_author) = parse_index_list_extended_record(&record)
            .with_context(|| format!("Failed to read record at {}: {:?}", i, &record))?;

        if let Some(existing_author) = authors.get(&author.id) {
            ensure!(
                existing_author == &author,
                "Different authors has same id:\n{:?}\n{:?}",
                &existing_author,
                &author
            );
        }

        authors.insert(author.id.clone(), author);

        if let Some(existing_book) = books.get(&book.id) {
            ensure!(
                existing_book == &book,
                "Different books has same id:\n{:?}\n{:?}",
                &existing_book,
                &book
            );
        }

        books.insert(book.id.clone(), book);

        ensure!(
            !book_authors.contains(&book_author),
            "Duplicate BookAuthor found: {:?}",
            &book_author
        );
        book_authors.insert(book_author);
    }

    let authors = authors.into_values().collect();
    let books = books.into_values().collect();
    let book_authors = book_authors.into_iter().collect();

    Ok(AozorabunkoIndexList {
        authors,
        books,
        book_authors,
    })
}

fn parse_index_list_extended_record(
    record: &csv::StringRecord,
) -> Result<(Author, Book, BookAuthor)> {
    let book_id = record[0].to_owned();
    let title = record[1].to_owned();
    let title_kana = record[2].to_owned();
    let sort_key = record[3].to_owned();
    let subtitle = record[4].to_owned();
    let subtitle_kana = record[5].to_owned();
    let original_title = record[6].to_owned();

    let writing_system = record[9].to_owned();

    let copyright = match &record[10] {
        "あり" => true,
        "なし" => false,
        _ => bail!("unknown work_copyright at {:?}", record),
    };

    let published_at = parse_date(&record[11])?;
    let updated_at = parse_date(&record[12])?;

    let author = {
        let author_id = record[14].to_owned();
        let last_name = record[15].to_owned();
        let first_name = record[16].to_owned();
        let last_name_kana = record[17].to_owned();
        let first_name_kana = record[18].to_owned();
        let last_name_sort_key = record[19].to_owned();
        let first_name_sort_key = record[20].to_owned();
        let last_name_romaji = record[21].to_owned();
        let first_name_romaji = record[22].to_owned();

        let birth_date = record[24].to_owned();
        let death_date = record[25].to_owned();

        let copyright = match &record[26] {
            "あり" => true,
            "なし" => false,
            _ => bail!("unknown work_copyright at {:?}", record),
        };

        Author {
            id: author_id,
            last_name,
            first_name,
            last_name_kana,
            first_name_kana,
            last_name_sort_key,
            first_name_sort_key,
            last_name_romaji,
            first_name_romaji,
            birth_date,
            death_date,
            copyright,
        }
    };

    let author_role = record[23].to_owned();

    let mut original_book = Vec::new();
    for i in &[27, 35] {
        let i = *i as usize;

        let title = record[i].to_owned();
        if title.is_empty() {
            continue;
        }

        let publisher_name = record[i + 1].to_owned();
        let first_edition_date = record[i + 2].to_owned();
        let input_edition = record[i + 3].to_owned();
        let proofreading_edition = record[i + 4].to_owned();

        let parent_title = record[i + 5].to_owned();
        let parent_publisher_name = record[i + 6].to_owned();
        let parent_first_edition_date = record[i + 7].to_owned();

        original_book.push(OriginalBook {
            title,
            publisher_name,
            first_edition_date,
            input_edition,
            proofreading_edition,
            parent_title,
            parent_publisher_name,
            parent_first_edition_date,
        })
    }

    let inputter_name = record[43].to_owned();
    let proofreader_name = record[44].to_owned();

    let book = Book {
        id: book_id,
        title,
        title_kana,
        sort_key,
        subtitle,
        subtitle_kana,
        original_title,
        writing_system,
        copyright,
        published_at,
        updated_at,
        original_book,
        inputter_name,
        proofreader_name,
    };

    let book_author = BookAuthor {
        book_id: book.id.clone(),
        author_id: author.id.clone(),
        author_role,
    };

    Ok((author, book, book_author))
}

fn parse_date(date: &str) -> Result<Date> {
    let date = date.replace(' ', ""); // 謎の空白を含む要素がある
    Date::parse(&date, &['-', '/'])
}
