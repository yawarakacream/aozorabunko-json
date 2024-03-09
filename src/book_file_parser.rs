use anyhow::{bail, ensure, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{accent_composer::compose_accent, utility::CharType};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BookContentOriginalDataType {
    RubyTxt,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookContent {
    pub original_data_type: BookContentOriginalDataType,
    pub header: Vec<BookContentElement>,
    pub body: Vec<BookContentElement>,
    pub footer: Vec<BookContentElement>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum BookContentElement {
    String { value: String, ruby: Option<String> },
    NewLine,

    KaipageAttention, // ［＃改ページ］
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type", content = "content")]
pub enum RubyTxtToken {
    String(String),
    NewLine,

    PositionStartDelimiter, // ｜

    RubyStart, // 《
    RubyEnd,   // 》

    GaijiAccentDecompositionStart, // 〔
    GaijiAccentDecompositionEnd,   // 〕
}

// 字句解析
pub fn tokenize_ruby_txt(txt: &str) -> Result<Vec<RubyTxtToken>> {
    let mut tokens = Vec::new();

    let mut chars: &[char] = &txt.chars().into_iter().collect::<Vec<_>>();

    let mut string_buffer = String::new();

    while !chars.is_empty() {
        let special_token = {
            match chars[0] {
                // 改行は公式に CR+LF とされているが完全には統一されていない
                '\r' => match chars.get(1) {
                    Some(&'\n') => Some((2, RubyTxtToken::NewLine)),
                    _ => Some((1, RubyTxtToken::NewLine)),
                },
                '\n' => Some((1, RubyTxtToken::NewLine)),

                '｜' => Some((1, RubyTxtToken::PositionStartDelimiter)),
                '《' => Some((1, RubyTxtToken::RubyStart)),
                '》' => Some((1, RubyTxtToken::RubyEnd)),

                '〔' => Some((1, RubyTxtToken::GaijiAccentDecompositionStart)),
                '〕' => Some((1, RubyTxtToken::GaijiAccentDecompositionEnd)),

                _ => None,
            }
        };

        match special_token {
            Some((len, token)) => {
                if !string_buffer.is_empty() {
                    tokens.push(RubyTxtToken::String(string_buffer));
                    string_buffer = String::new();
                }

                tokens.push(token);
                chars = &chars[len..];
            }

            None => {
                string_buffer.push(chars[0]);
                chars = &chars[1..];
            }
        }
    }

    Ok(tokens)
}

// 構文解析
pub fn parse_ruby_txt(tokens: &[RubyTxtToken]) -> Result<BookContent> {
    ensure!(!tokens.is_empty(), "Cannot parse empty array");

    let mut tokens = tokens;

    // 冒頭
    let header = {
        ensure!(
            !matches!(tokens[0], RubyTxtToken::NewLine),
            "Header starts with empty line"
        );

        let mut header_tokens = Vec::new();

        loop {
            let token = tokens.get(0).context("Failed to load header")?;
            tokens = &tokens[1..];

            if token == &RubyTxtToken::NewLine && tokens.get(0) == Some(&RubyTxtToken::NewLine) {
                break;
            }

            header_tokens.push(token);
        }

        let mut elements = parse_block(&header_tokens)?;

        // 最後の空行を消す
        while let Some(last) = elements.last() {
            if !matches!(last, BookContentElement::NewLine) {
                break;
            }
            elements.pop();
        }
        ensure!(!elements.is_empty(), "Header is empty");

        for el in &elements {
            ensure!(
                matches!(
                    el,
                    BookContentElement::String { value: _, ruby: _ } | BookContentElement::NewLine
                ),
                "Invalid element is found in header: {:?}",
                el
            );
        }

        elements
    };

    // 冒頭から本文の間の空白行を飛ばす
    while tokens.get(0).context("Body is empty")? == &RubyTxtToken::NewLine {
        tokens = &tokens[1..];
    }

    let body = {
        // 基本は "底本："
        static REGEX_FOOTER_CHECKER: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^底本()?[：:「]").unwrap());

        // static REGEX_ANNOTATION_DESCRIPTION: Lazy<Regex> =
        //     Lazy::new(|| Regex::new(r"^[\s　]*[【《]テキスト中に現れる記号について").unwrap());

        static REGEX_ANNOTATION_DESCRIPTION: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^【テキスト中に現れる記号について】$").unwrap());

        let mut blocks = vec![vec![]];
        loop {
            let token = tokens.get(0).context("Failed to load body")?;
            tokens = &tokens[1..];

            if let RubyTxtToken::String(string) = token {
                // 主に "【テキスト中に現れる記号について】" を表す区切り
                // その他にも単なる区切りとして使われることもある（改ページ？）
                // 個数は一定でない
                // この区切りで表されるものをブロックと呼ぶ
                if string.chars().into_iter().all(|c| c == '-') {
                    if !blocks.last().unwrap().is_empty() {
                        blocks.push(vec![]);
                    }
                    continue;
                }

                if REGEX_FOOTER_CHECKER.is_match(&string) {
                    break;
                }
            }

            blocks.last_mut().unwrap().push(token);
        }

        // 長ハイフン (REGEX_ALL_HYPHEN) を footer の区切りにしているものがある
        if blocks.last().unwrap().is_empty() {
            blocks.pop();
        }

        let mut elements = Vec::new();

        for block in blocks {
            // ブロックの境は改ページにする
            if let Some(last) = elements.last() {
                if !matches!(last, BookContentElement::KaipageAttention) {
                    elements.push(BookContentElement::KaipageAttention);
                }
            }

            // 前後の空行を削除
            let start_index = block
                .iter()
                .position(|&token| !matches!(token, RubyTxtToken::NewLine))
                .context("Empty block is found")?;
            let end_index = block.len()
                - block
                    .iter()
                    .rev()
                    .position(|&token| !matches!(token, RubyTxtToken::NewLine))
                    .unwrap();
            let block = &block[start_index..end_index];
            if block.is_empty() {
                continue;
            }

            if let Some(RubyTxtToken::String(value)) = block.first() {
                // 注記の説明のページは飛ばす
                if REGEX_ANNOTATION_DESCRIPTION.is_match(&value) {
                    continue;
                }
            }

            let sub_elements = parse_block(block)?;

            elements.extend(sub_elements);
        }

        // 最後の空行を消す
        while let Some(last) = elements.last() {
            if !matches!(last, BookContentElement::NewLine) {
                break;
            }
            elements.pop();
        }
        ensure!(!elements.is_empty(), "Body is empty");

        elements
    };

    // 本文から末尾の間の空白行を飛ばす
    while tokens.get(0).context("Footer is empty")? == &RubyTxtToken::NewLine {
        tokens = &tokens[1..];
    }

    let footer = {
        let footer_tokens = tokens.iter().map(|t| t).collect::<Vec<_>>();
        let mut elements = parse_block(&footer_tokens)?;

        // 最後の空行を消す
        while let Some(last) = elements.last() {
            if !matches!(last, BookContentElement::NewLine) {
                break;
            }
            elements.pop();
        }
        ensure!(!elements.is_empty(), "Footer is empty");

        for el in &elements {
            ensure!(
                matches!(
                    el,
                    BookContentElement::String { value: _, ruby: _ } | BookContentElement::NewLine
                ),
                "Invalid element is found in footer: {:?}",
                el
            );
        }

        elements
    };

    Ok(BookContent {
        original_data_type: BookContentOriginalDataType::RubyTxt,
        header,
        body,
        footer,
    })
}

struct BookContentElementList {
    elements: Vec<BookContentElement>,
    string_buffer: String,
}

impl BookContentElementList {
    pub fn new() -> Self {
        BookContentElementList {
            elements: Vec::new(),
            string_buffer: String::new(),
        }
    }

    // pub fn first(&mut self) -> Option<&BookContentElement> {
    //     self.elements.first()
    // }

    // pub fn last(&mut self) -> Option<&BookContentElement> {
    //     self.elements.last()
    // }

    pub fn pop(&mut self) -> Option<BookContentElement> {
        self.elements.pop()
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn push(&mut self, element: BookContentElement) {
        self.apply_string_buffer();
        self.elements.push(element);
    }

    pub fn push_char(&mut self, value: char) {
        self.string_buffer.push(value)
    }

    pub fn push_str(&mut self, value: &str) {
        self.string_buffer.push_str(&value);
    }

    pub fn apply_string_buffer(&mut self) {
        if self.string_buffer.is_empty() {
            return;
        }

        let string_buffer = self.string_buffer.clone();
        self.elements.push(BookContentElement::String {
            value: string_buffer,
            ruby: None,
        });

        self.string_buffer.clear();
    }

    pub fn to_vec(mut self) -> Vec<BookContentElement> {
        self.apply_string_buffer();
        self.elements
    }
}

impl<Idx> std::ops::Index<Idx> for BookContentElementList
where
    Idx: std::slice::SliceIndex<[BookContentElement], Output = BookContentElement>,
{
    type Output = BookContentElement;

    #[inline(always)]
    fn index(&self, index: Idx) -> &Self::Output {
        self.elements.index(index)
    }
}

// 構文解析（本文）
fn parse_block(tokens: &[&RubyTxtToken]) -> Result<Vec<BookContentElement>> {
    let mut tokens = tokens;
    let mut elements = BookContentElementList::new();

    // PositionStartDelimiter が挿入されていた所の次のトークンを指す
    let mut position_start_delimiter_index = None;

    while !tokens.is_empty() {
        let token0 = tokens[0];
        tokens = &tokens[1..];

        match token0 {
            RubyTxtToken::String(value) => elements.push_str(value),
            RubyTxtToken::NewLine => elements.push(BookContentElement::NewLine),

            RubyTxtToken::PositionStartDelimiter => {
                if position_start_delimiter_index.is_some() {
                    bail!("Cannot write '｜' consecutively")
                }
                elements.apply_string_buffer();
                position_start_delimiter_index = Some(elements.len() + 1);
            }

            RubyTxtToken::RubyStart => {
                // ファイル最初 or 行頭の '《' はルビにしない方がいい？

                let mut end_index = None;
                for (i, &token) in tokens.iter().enumerate() {
                    match token {
                        &RubyTxtToken::RubyEnd => {
                            end_index = Some(i);
                            break;
                        }
                        &RubyTxtToken::NewLine => break,
                        _ => continue,
                    }
                }

                let end_index = end_index.context("A line ends without '》'")?;

                let ruby = {
                    let child_tokens = &tokens[..end_index];
                    let child_elements = parse_block(&child_tokens)?;

                    // 中身のない "《》" はルビにしない
                    if child_elements.is_empty() {
                        elements.push_char('《');
                        continue;
                    }

                    ensure!(
                        child_elements.len() == 1,
                        "Invalid ruby: {:?}",
                        child_elements
                    );

                    match &child_elements[0] {
                        BookContentElement::String {
                            value: child_value,
                            ruby: None,
                        } => child_value.clone(),
                        el => bail!("Invalid element is found in Ruby: {:?}", el),
                    }
                };

                elements.apply_string_buffer();

                // ルビを振る
                match position_start_delimiter_index {
                    Some(psd_index) => {
                        ensure!(psd_index == elements.len(), "Invalid delimiter");

                        match elements.pop().context("Cannot set ruby to None")? {
                            BookContentElement::String { value, ruby: ruby0 } => {
                                ensure!(!value.is_empty(), "Cannot set ruby to empty String");
                                ensure!(ruby0.is_none(), "Cannot set 2 rubies to 1 String");

                                elements.push(BookContentElement::String {
                                    value,
                                    ruby: Some(ruby),
                                });
                            }

                            el => bail!("Cannot set ruby to {:?}", el),
                        }

                        position_start_delimiter_index = None;
                    }

                    None => match elements.pop().context("Cannod set ruby to None")? {
                        BookContentElement::String { value, ruby: ruby0 } => {
                            ensure!(!value.is_empty(), "Cannot set ruby to empty String");
                            ensure!(ruby0.is_none(), "Cannot set 2 rubies to 1 String");

                            let value_chars: Vec<_> = value.chars().collect();

                            let mut ruby_start_index = value_chars.len();
                            let last_char_type = CharType::from(*value_chars.last().unwrap());
                            for c in value_chars.iter().rev() {
                                if CharType::from(*c) != last_char_type {
                                    break;
                                }
                                ruby_start_index -= 1;
                            }

                            if 0 < ruby_start_index {
                                elements.push(BookContentElement::String {
                                    value: value_chars[..ruby_start_index].iter().collect(),
                                    ruby: None,
                                });
                            }
                            elements.push(BookContentElement::String {
                                value: value_chars[ruby_start_index..].iter().collect(),
                                ruby: Some(ruby),
                            });
                        }

                        el => bail!("Cannot set ruby to {:?}", el),
                    },
                }

                tokens = &tokens[(end_index + 1)..];
            }
            // 対応する '《' があったならここに来ないので '》' を入れる
            RubyTxtToken::RubyEnd => elements.push_char('》'),

            RubyTxtToken::GaijiAccentDecompositionStart => {
                let mut end_index = None;
                for (i, &token) in tokens.iter().enumerate() {
                    match token {
                        &RubyTxtToken::GaijiAccentDecompositionEnd => {
                            end_index = Some(i);
                            break;
                        }
                        &RubyTxtToken::NewLine => break,
                        _ => continue,
                    }
                }

                if let Some(end_index) = end_index {
                    let child_tokens = &tokens[..end_index];
                    let mut child_tokens: Vec<RubyTxtToken> =
                        child_tokens.iter().map(|&t| t.clone()).collect();

                    let mut composed = false;
                    for i in 0..child_tokens.len() {
                        if let RubyTxtToken::String(value) = &child_tokens[i] {
                            let new_value = compose_accent(&value);
                            if value != &new_value {
                                composed = true;
                                child_tokens[i] = RubyTxtToken::String(new_value);
                            }
                        }
                    }

                    // 合成したならば合成後のトークンで構文解析する
                    if composed {
                        let child_tokens = &child_tokens.iter().map(|t| t).collect::<Vec<_>>();
                        let child_elements = parse_block(&child_tokens)?;
                        child_elements.into_iter().for_each(|el| elements.push(el));
                        tokens = &tokens[(end_index + 1)..];
                        continue;
                    }
                }

                // 対応する '〕' が見つからない or 見つかったがアクセント分解の合成を行わなかったならば '〔' を入れる
                elements.push_char('〔');
            }
            // 対応する '〔' があったならここに来ないので '〕' を入れる
            RubyTxtToken::GaijiAccentDecompositionEnd => elements.push_char('〕'),
        }
    }

    Ok(elements.to_vec())
}
