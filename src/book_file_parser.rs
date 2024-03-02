use anyhow::{Context, Ok, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use crate::utility::trim_empty_lines;

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BookContentOriginalType {
    RubyTxt,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookContent {
    pub original_type: BookContentOriginalType,
    pub header: Vec<String>,
    pub body: Vec<Vec<BookLine>>,
    pub footer: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BookLine {
    pub contents: Vec<BookLineContent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum BookLineContent {
    String { value: String, ruby: Option<String> },
    Annotation {},
}

// 青空文庫 注記一覧 https://www.aozora.gr.jp/annotation/ のフォーマットに従った解析
// 2010 年 4 月 1 日公布
//
// 公布日以降の作品の多くはこのフォーマットに従っている
//
// - 冒頭・末尾について規格が定められているが、昔のものはそれに沿っていない
//   - (例) https://www.aozora.gr.jp/cards/000168/card909.html
//     - 冒頭が "タイトル\n\n著者\n\n本文"
//     - 末尾が "底本：" ではなく "入力者注"
// - 長いハイフンは "テキスト中に現れる記号について" を示すためとされているが
//   単なる区切り？としての利用もある
//   - (例) https://www.aozora.gr.jp/cards/000124/card652.html
pub fn parse_ruby_txt(txt: &str) -> Result<BookContent> {
    // 公式に CR+LF とされているが完全には統一されていない
    static REGEX_NEW_LINE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\r\n|\n|\r").unwrap());

    let mut txt = REGEX_NEW_LINE.split(txt).into_iter().peekable();

    // 冒頭
    let header = {
        let mut header = Vec::new();
        loop {
            let line = txt.next().context("Failed to load header")?;
            if line.is_empty() {
                break;
            }

            header.push(line);
        }

        let header = header.into_iter().map(|t| t.to_owned()).collect();

        header
    };

    let body = {
        // 区切りのハイフンの個数は一定でない
        static REGEX_ALL_HYPHEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\-+$").unwrap());

        // 基本は "底本："
        static REGEX_FOOTER_CHECKER: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^底本(・初出)?(：|:)").unwrap());

        let mut body = vec![vec![]];

        loop {
            let &line = txt.peek().context("Text ends without footer")?;

            if REGEX_FOOTER_CHECKER.is_match(line) {
                break;
            }

            txt.next();

            // 主に "【テキスト中に現れる記号について】" への対応
            // その他にも単なる（ページの？）区切りとして使われることもある
            if REGEX_ALL_HYPHEN.is_match(line) {
                if !body.last().unwrap().is_empty() {
                    body.push(vec![]);
                }
            } else {
                body.last_mut().unwrap().push(line);
            }
        }

        let body = body
            .into_iter()
            .map(|mut block| {
                trim_empty_lines(&mut block);

                block
                    .into_iter()
                    .map(|line| BookLine {
                        contents: vec![BookLineContent::String {
                            value: line.to_string(),
                            ruby: None,
                        }],
                    })
                    .collect()
            })
            .collect();

        body
    };

    let footer = {
        let mut footer = txt.collect();

        trim_empty_lines(&mut footer);

        let footer = footer.into_iter().map(|t| t.to_owned()).collect();

        footer
    };

    Ok(BookContent {
        original_type: BookContentOriginalType::RubyTxt,
        header,
        body,
        footer,
    })
}
