use anyhow::{bail, ensure, Context, Ok, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use crate::{
    accent_composer::compose_accent,
    jis_x_0213,
    utility::{trim_empty_lines, CharType},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BookContentOriginalDataType {
    RubyTxt,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookContent {
    pub original_data_type: BookContentOriginalDataType,
    pub header: Vec<String>,
    pub body: Vec<Vec<BookLine>>,
    pub footer: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BookLine {
    pub contents: Vec<BookLineContent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
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

    // 冒頭から本文の間の空白行を飛ばす
    while txt.peek().context("Body is missing")?.is_empty() {
        txt.next();
    }

    let body = {
        // 主に "【テキスト中に現れる記号について】" を表す区切り
        // その他にも単なる（ページの？）区切りとして使われることもある
        // 個数は一定でない
        static REGEX_ALL_HYPHEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\-+$").unwrap());

        // 基本は "底本："
        static REGEX_FOOTER_CHECKER: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^底本()?(：|:)").unwrap());

        static REGEX_ANNOTATION_DESCRIPTION: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^(\s|　)*(【|《)テキス.中に現れる記号について(】|》)(\s|　)*$").unwrap()
        });

        let mut body = vec![vec![]];

        while let Some(&line) = txt.peek() {
            if REGEX_FOOTER_CHECKER.is_match(line) {
                break;
            }

            if line == "［＃本文終わり］" {
                txt.next();
                break;
            }

            txt.next();

            if REGEX_ALL_HYPHEN.is_match(line) {
                if !body.last().unwrap().is_empty() {
                    body.push(vec![]);
                }
            } else {
                body.last_mut().unwrap().push(line);
            }
        }

        // 長ハイフン (REGEX_ALL_HYPHEN) を footer の区切りにしているものがある
        if body.last().unwrap().is_empty() {
            body.pop();
        }

        let body = {
            let mut new_body = Vec::with_capacity(body.len());

            for block in &mut body {
                trim_empty_lines(block);

                ensure!(!block.is_empty());

                if REGEX_ANNOTATION_DESCRIPTION.is_match(block[0]) {
                    let mut new_block = Vec::with_capacity(block.len());
                    for line in block {
                        new_block.push(BookLine {
                            contents: vec![BookLineContent::String {
                                value: line.to_string(),
                                ruby: None,
                            }],
                        });
                    }
                    new_body.push(new_block);
                    continue;
                }

                let mut new_block = Vec::new();
                for line in block {
                    let line = parse_line(line)
                        .with_context(|| format!("Failed to parse line {:?}", line))?;
                    new_block.push(line);
                }
                new_body.push(new_block);
            }
            new_body
        };

        body
    };

    let footer = {
        let mut footer = txt.collect();

        trim_empty_lines(&mut footer);

        let footer = footer.into_iter().map(|t| t.to_owned()).collect();

        footer
    };

    Ok(BookContent {
        original_data_type: BookContentOriginalDataType::RubyTxt,
        header,
        body,
        footer,
    })
}

fn parse_line(line: &str) -> Result<BookLine> {
    if line.is_empty() {
        return Ok(BookLine {
            contents: Vec::new(),
        });
    }

    // 外字（第 1, 2 水準にない漢字）
    let line = {
        static REGEX_GAIJI: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"※［＃「.+?」、第[0-9]水準(?P<plane>[0-9]+)-(?P<row>[0-9]+)-(?P<cell>[0-9]+)］?",
            )
            .unwrap()
        });

        REGEX_GAIJI
            .replace_all(line, |caps: &regex::Captures<'_>| {
                let plane = caps.name("plane").unwrap().as_str().parse().unwrap();
                let row = caps.name("row").unwrap().as_str().parse().unwrap();
                let cell = caps.name("cell").unwrap().as_str().parse().unwrap();
                jis_x_0213::JIS_X_0213.get(&(plane, row, cell)).unwrap()
            })
            .into_owned()
    };

    // 外字（アクセント分解）
    // アクセント分解したものにルビが振られていることがあるので先に処理する
    let line = {
        let mut chars = line.chars().into_iter().peekable();

        let mut line = String::new();

        while let Some(c) = chars.next() {
            match c {
                '〔' => {
                    let mut s0 = String::new();
                    let mut try_compose = false;
                    while let Some(c) = chars.next() {
                        if c == '〕' {
                            try_compose = true;
                            break;
                        }
                        s0.push(c);
                    }

                    // '〕' なしで行が終わったときはそのまま
                    if !try_compose {
                        line.push('〔');
                        line.push_str(&s0);
                        break;
                    }

                    let s1 = compose_accent(&s0);

                    // 分解結果が変わらなければ括弧も含めて戻す
                    if s0 == s1 {
                        line.push('〔');
                        line.push_str(&s0);
                        line.push('〕');
                    } else {
                        line.push_str(&s1);
                    }
                }
                _ => line.push(c),
            }
        }

        line
    };

    let mut contents = Vec::new();

    let mut chars = line.chars().into_iter().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            // ルビ
            ('《', _) => {
                let mut ruby = String::new();
                loop {
                    let c = chars.next().context("A line ends without '》'")?;
                    if c == '》' {
                        break;
                    }
                    ruby.push(c);
                }

                match contents.pop().context("No element found to add ruby")? {
                    BookLineContent::String { value, ruby: ruby0 } => {
                        ensure!(!value.is_empty(), "Cannot add ruby to empty String");
                        ensure!(ruby0.is_none(), "Cannot add 2 rubies to 1 String");

                        let mut chars: Vec<_> = value.chars().collect();

                        let ruby_start_index = {
                            let mut ruby_start_char_index = None;
                            for (i, c) in chars.iter().enumerate().rev() {
                                if c == &'｜' {
                                    ensure!(
                                        ruby_start_char_index.is_none(),
                                        "Duplicate '｜' are found at {} and {} in {:?}",
                                        ruby_start_char_index.unwrap(),
                                        i,
                                        value
                                    );
                                    ruby_start_char_index = Some(i);
                                }
                            }

                            if let Some(ruby_start_char_index) = ruby_start_char_index {
                                ensure!(chars.remove(ruby_start_char_index) == '｜');
                                ruby_start_char_index
                            } else {
                                let mut ruby_start_index = chars.len();
                                let last_char_type = CharType::from(*chars.last().unwrap());
                                for c in chars.iter().rev() {
                                    if CharType::from(*c) != last_char_type {
                                        break;
                                    }
                                    ruby_start_index -= 1;
                                }
                                ruby_start_index
                            }
                        };

                        if 0 < ruby_start_index {
                            contents.push(BookLineContent::String {
                                value: chars[..ruby_start_index].iter().collect(),
                                ruby: None,
                            });
                        }
                        contents.push(BookLineContent::String {
                            value: chars[ruby_start_index..].iter().collect(),
                            ruby: Some(ruby),
                        });
                    }
                    _ => {
                        bail!("Cannot add character to non-String content")
                    }
                }
            }

            _ => match contents.last_mut() {
                Some(BookLineContent::String { value, ruby: None }) => value.push(c),
                _ => contents.push(BookLineContent::String {
                    value: c.to_string(),
                    ruby: None,
                }),
            },
        }
    }

    // validate
    match &contents.last() {
        Some(last) => match last {
            BookLineContent::String { value, ruby: _ } => {
                ensure!(!value.is_empty(), "last of contents is empty");
            }
            _ => {}
        },
        None => {}
    }

    Ok(BookLine { contents })
}
