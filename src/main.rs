pub mod jis_x_0213;
pub mod list_person_all_extended_csv;
pub mod ruby_txt;
pub mod utility;

use anyhow::{bail, ensure, Context, Result};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    path::PathBuf,
};

use crate::{
    list_person_all_extended_csv::parser::parse_list_person_all_extended_csv,
    ruby_txt::{parser::parse_ruby_txt, tokenizer::tokenize_ruby_txt},
    utility::ZipReader,
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

        parse_list_person_all_extended_csv(&csv_data)?
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

    // 著作権がある本の ID
    let mut book_ids_with_copyright = HashSet::new();
    for ba in aozorabunko_index_list.book_authors {
        if author_ids_with_copyright.contains(&ba.author_id) {
            book_ids_with_copyright.insert(ba.book_id);
        }
    }
    let book_ids_with_copyright = book_ids_with_copyright;

    let pb = create_progress_bar(aozorabunko_index_list.books.len() as u64);
    for book in aozorabunko_index_list.books.iter().progress_with(pb) {
        // テキストファイルにミスがあるものは飛ばす
        if [
            // "【テキスト中に現れる記号について】" が "《テキスト中に現れる記号について》" になっている
            18379, // 楠山正雄「くらげのお使い」
            45670, // 林不忘「魔像」
            45664, // 福沢諭吉「旧藩情」
            46228, // 林不忘「巷説享保図絵」
            46229, // 林不忘「つづれ烏羽玉」
            //
            // "底本：" のミス
            1871, // エドガー・アラン・ポー「落穴と振子」　"底本「"
            2526, // エドガー・アラン・ポー「早すぎる埋葬」　"底本「"
            //
            // 不明な書式
            395,   // 萩原朔太郎「散文詩集『田舎の時計　他十二篇』」
            455,   // 宮沢賢治「ガドルフの百合」
            906,   // 横光利一「時間」
            909,   // 横光利一「鳥」
            1255,  // 海野十三「海野十三敗戦日記」　謎 annotation
            4832,  // 宮本百合子「日記」『一九一三年（大正二年）』　謎 annotation
            46237, // 宮本百合子「日記」『一九一七年（大正六年）』　謎 annotation
            46241, // 宮本百合子「日記」『一九二二年（大正十一年）』　謎 annotation
            46244, // 宮本百合子「日記」『一九二六年（大正十五年・昭和元年）』　謎 annotation
            46247, // 宮本百合子「日記」『一九二九年（昭和四年）』　謎 annotation
            //
            // 細かいミス
            351, // 三遊亭圓朝「業平文治漂流奇談」　ルビの中に注記 "過《あやま［＃「ま」は、底本では欠如］》り"
            1490, // 三遊亭圓朝「西洋人情話英国孝子ジョージスミス之伝」　ルビの中に注記 "願掛《がんがけ［＃底本では「け」が脱落］》"
            2168, // 與謝野寛、與謝野晶子「巴里より」　"一番向｜《むか》うにある"
            2218, // 若山牧水「樹木とその葉」　"しん［＃「しん」傍点］"
            2559, // 與謝野晶子「遺書」　"態々《わざ／＼［＃底本では「／＼」は「／″＼」と誤植］》"
            24456, // 南方熊楠「棄老傳説に就て」　"底本・" が "底本・初出："
            42687, // 三遊亭圓朝「後の業平文治」　ルビの中に注記 "長大小《なが［＃「なが」は底本では「なだ」と誤記］だいしょう》"
            43035, // 岡本かの子「花は勁し」　"底本" が "定本" になっている
            56634, // 梅崎春生「幻化」　"「もう一杯｜《く》呉れ」"
        ]
        .contains(&book.id)
        {
            continue;
        }

        // aozorabunko-json が未対応のものは飛ばす
        if [
            1897,  // 正岡子規「墨汁一滴」　不明な外字 "※［＃「麾−毛」、42-8］"
            2032, // 宮本百合子「風に乗って来るコロポックル」　"《シサム》［＃「ム」は小書き片仮名ム、1-6-89］"
            47202, // 折口信夫「用言の発展」　"※［＃ハングル文字、「ロ／亅／一」、439-17］"
            51729, // 「古事記」　不明な外字 "※［＃「討／貝」、406-2-9］"
        ]
        .contains(&book.id)
        {
            continue;
        }

        // 著作権があるものは飛ばす
        if book_ids_with_copyright.contains(&book.id) {
            continue;
        }

        let book_directory_path = book_root_path.join(book.id.to_string());
        fs::create_dir_all(&book_directory_path).unwrap();

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
                    let content = tokenize_ruby_txt(&txt).and_then(|x| parse_ruby_txt(&x))?;
                    fs::write(
                        &book_directory_path.join("content_from_ruby-txt.json"),
                        serde_json::to_string_pretty(&content)?,
                    )?;
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
