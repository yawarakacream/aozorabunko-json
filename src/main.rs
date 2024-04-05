use anyhow::{bail, ensure, Context, Result};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    path::PathBuf,
};

use aozorabunko_json::{
    list_person_all_extended_csv::parser::{
        parse_list_person_all_extended_csv, AozorabunkoIndexList,
    },
    ruby_txt::{
        parser::{parse_ruby_txt, ParsedRubyTxt},
        renderer::{render_ruby_txt, RenderedRubyTxt},
        tokenizer::tokenize_ruby_txt,
    },
    utility::zip::ZipReader,
};

struct Args {
    aozorabunko_path: String,
    output_path: Option<String>,
}

fn get_args() -> Result<Args> {
    let args: Vec<String> = env::args().skip(1).collect();

    let opts = getopts::Options::new();

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => bail!(f),
    };

    let aozorabunko_path = matches
        .free
        .get(0)
        .context("path to aozorabunko repository is required")?
        .clone();
    let output_path = matches.free.get(1).map(|s| s.clone());

    Ok(Args {
        aozorabunko_path,
        output_path,
    })
}

// bad practice?
enum BuildOut {
    Null,
    File { root: PathBuf },
}

impl BuildOut {
    fn init_file(root: &str) -> Result<Self> {
        let root = PathBuf::from(&root);
        fs::create_dir(&root).context("Failed to create output directory")?;

        Ok(Self::File { root })
    }

    fn save_aozorabunko_index_list(
        &self,
        aozorabunko_index_list: &AozorabunkoIndexList,
    ) -> Result<()> {
        if let BuildOut::File { root } = &self {
            fs::write(
                &root.join("books.json"),
                serde_json::to_string(&aozorabunko_index_list.books)?,
            )?;

            fs::write(
                &root.join("authors.json"),
                serde_json::to_string(&aozorabunko_index_list.authors)?,
            )?;

            fs::write(
                &root.join("book_authors.json"),
                serde_json::to_string(&aozorabunko_index_list.book_authors)?,
            )?;
        }

        Ok(())
    }

    fn save_book_ruby_txt(
        &self,
        book_id: usize,
        parsed: &ParsedRubyTxt,
        rendered: &RenderedRubyTxt,
    ) -> Result<()> {
        if let BuildOut::File { root } = &self {
            let book_directory_path = &root.join(format!("book/{}", book_id));
            fs::create_dir_all(&book_directory_path).unwrap();

            fs::write(
                &book_directory_path.join("ruby-txt_parsed.json"),
                serde_json::to_string(&parsed).unwrap(),
            )
            .unwrap();

            fs::write(
                &book_directory_path.join("ruby-txt_rendered.json"),
                serde_json::to_string(&rendered).unwrap(),
            )
            .unwrap();
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let args = get_args()?;

    let aozorabunko_path = PathBuf::from(&args.aozorabunko_path);
    ensure!(
        aozorabunko_path.exists(),
        "File not found: {}",
        aozorabunko_path.display()
    );

    let out = if let Some(output_path) = &args.output_path {
        BuildOut::init_file(&output_path)
            .with_context(|| format!("Failed to output directory: {}", &output_path))?
    } else {
        BuildOut::Null
    };

    println!("Processing list_person_all_extended...");

    let aozorabunko_index_list = {
        let csv_zip_path = aozorabunko_path.join("index_pages/list_person_all_extended_utf8.zip");
        let csv_zip_file = File::open(csv_zip_path).unwrap();
        let mut csv_zip_reader = ZipReader::new(csv_zip_file)?;

        let mut csv_entry = csv_zip_reader.get_by_path("list_person_all_extended_utf8.csv")?;
        let csv_data = csv_entry.as_string()?;

        parse_list_person_all_extended_csv(&csv_data)?
    };

    out.save_aozorabunko_index_list(&aozorabunko_index_list)?;

    println!("Finished.");

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
        // 著作権があるものは飛ばす
        if book_ids_with_copyright.contains(&book.id) {
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
                    let tokens = tokenize_ruby_txt(&txt).context("Failed to tokenize")?;

                    if is_supported_to_parse(&book.id) {
                        let parsed = parse_ruby_txt(&tokens).context("Failed to parse")?;

                        if is_supported_to_render(&book.id) {
                            let rendered = render_ruby_txt(&parsed).context("Failed to render")?;

                            out.save_book_ruby_txt(book.id, &parsed, &rendered)?;
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

fn is_supported_to_parse(book_id: &usize) -> bool {
    ![
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
        2168,  // 與謝野寛、與謝野晶子「巴里より」　"一番向｜《むか》うにある"
        2218,  // 若山牧水「樹木とその葉」　"しん［＃「しん」傍点］"
        24456, // 南方熊楠「棄老傳説に就て」　"底本・" が "底本・初出："
        43035, // 岡本かの子「花は勁し」　"底本" が "定本" になっている
        56634, // 梅崎春生「幻化」　"「もう一杯｜《く》呉れ」"
        //
        // aozorabunko-json が未対応
        1317,  // 小栗虫太郎「黒死館殺人事件」　画像にルビ
        1897,  // 正岡子規「墨汁一滴」　不明な外字 "※［＃「麾−毛」、42-8］"
        2032, // 宮本百合子「風に乗って来るコロポックル」　"《シサム》［＃「ム」は小書き片仮名ム、1-6-89］"
        47202, // 折口信夫「用言の発展」　"※［＃ハングル文字、「ロ／亅／一」、439-17］"
        51729, // 「古事記」　不明な外字 "※［＃「討／貝」、406-2-9］"
    ]
    .contains(book_id)
}

fn is_supported_to_render(book_id: &usize) -> bool {
    ![
        // 細かいミス
        2590,  // 倉田百三「愛と認識との出発」　地寄せの記述ミス
        2733,  // 宮本百合子「ソヴェトの芝居」　地付きの記述ミス
        44907, // 桑原隲藏「支那の孝道殊に法律上より觀たる支那の孝道」　"［＃ここで字下げ終わり］" の前に謎の空白
        53104, // 柳田国男「木綿以前の事」　"［＃５字下げ］" の前に謎の空白
        57532, // 江戸川乱歩「新宝島」　"［＃３字下げ］" の前に謎の空白
        58209, // 野村胡堂「銭形平次捕物控」　"［＃７字下げ］" の前に謎の空白
        //
        // 不明な書式
        56258, // 山崎富栄「雨の玉川心中」　"　　十一月三十日［＃１１字下げ］富栄"
        57464, // 中谷宇吉郎「冬彦夜話」　"［＃ここで字下げ終わり］" が独立した行でない
        60609, // 上田秋成（鵜月洋訳）「雨月物語」『現代語訳　雨月物語』　"［＃１字下げ］書肆［＃地から３字上げ］"
        //
        // aozorabunko-json が未対応
        4462,  // 宮沢賢治「文語詩稿　一百篇」　"［＃改ページ］" についての説明が入っている
        49825, // 下村湖人「青年の思索のために」　1 行に 2 つのブロック終わり注記 "［＃ここで小さな文字終わり］［＃ここで字下げ終わり］"
        55342, // 野村長一「名曲決定盤」　"［＃改ページ］" についての説明が入っている
    ]
    .contains(book_id)
}
