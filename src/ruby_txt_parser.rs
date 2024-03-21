// 青空文庫 注記一覧 https://www.aozora.gr.jp/annotation/（2010 年 4 月 1 日公布）のフォーマットに従った解析
//
// フォーマットから外れたものは基本的にエラーとするが，一部フールプルーフする：
// - 改行は公式に CR+LF とされているが完全には統一されていない
// - "底本：" の "底本" と '：' の間に文字があってもよい
// - 長いハイフンは "テキスト中に現れる記号について" を示すためとされているが
//   単なる区切り？としての利用もある
//   - (例) https://www.aozora.gr.jp/cards/000124/card652.html

use anyhow::{bail, ensure, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    accent_composer::compose_accent,
    book_content::{
        BookContent, BookContentElement, BookContentElementList, BookContentOriginalDataType,
    },
    jis_x_0213,
    utility::CharType,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type", content = "content")]
pub enum RubyTxtToken {
    String(String),
    Kunojiten { dakuten: bool },
    NewLine,

    PositionStartDelimiter, // ｜

    RubyStart, // 《
    RubyEnd,   // 》

    AnnotationStart, // ［＃
    AnnotationEnd,   // ］

    GaijiAnnotationStart, // ※［＃

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
                '／' => match chars.get(1) {
                    Some(&'＼') => Some((2, RubyTxtToken::Kunojiten { dakuten: false })),
                    Some(&'″') => match chars.get(2) {
                        Some(&'＼') => Some((3, RubyTxtToken::Kunojiten { dakuten: true })),
                        _ => None,
                    },
                    _ => None,
                },

                // 改行は公式に CR+LF とされているが完全には統一されていない
                '\r' => match chars.get(1) {
                    Some(&'\n') => Some((2, RubyTxtToken::NewLine)),
                    _ => Some((1, RubyTxtToken::NewLine)),
                },
                '\n' => Some((1, RubyTxtToken::NewLine)),

                '｜' => Some((1, RubyTxtToken::PositionStartDelimiter)),
                '《' => Some((1, RubyTxtToken::RubyStart)),
                '》' => Some((1, RubyTxtToken::RubyEnd)),

                '［' => match chars.get(1) {
                    Some(&'＃') => Some((2, RubyTxtToken::AnnotationStart)),
                    _ => None,
                },
                '］' => Some((1, RubyTxtToken::AnnotationEnd)),

                '※' => match (chars.get(1), chars.get(2)) {
                    (Some(&'［'), Some(&'＃')) => Some((3, RubyTxtToken::GaijiAnnotationStart)),
                    _ => None,
                },

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
        // "底本："
        static REGEX_FOOTER_CHECKER: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^底本()?[：:「]").unwrap());

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
                if value == "【テキスト中に現れる記号について】" {
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

// 構文解析
fn parse_block<'a>(tokens: &'a [&'a RubyTxtToken]) -> Result<Vec<BookContentElement>> {
    let mut tokens = tokens;
    let mut elements = BookContentElementList::new();

    while !tokens.is_empty() {
        match tokens[0] {
            RubyTxtToken::String(value) => {
                tokens = &tokens[1..];
                elements.push_str(value);
            }

            RubyTxtToken::Kunojiten { dakuten } => {
                tokens = &tokens[1..];
                elements.push_char(if *dakuten { '〲' } else { '〱' });
            }

            RubyTxtToken::NewLine => {
                tokens = &tokens[1..];
                elements.push(BookContentElement::NewLine);
            }

            RubyTxtToken::PositionStartDelimiter => match parse_delimiter_and_tokens(tokens)? {
                ParsedDelimiterAndTokens::NotDelimiter => {
                    tokens = &tokens[1..];
                    elements.push_char('｜');
                }
                ParsedDelimiterAndTokens::Element(t, child) => {
                    tokens = t;
                    elements.push(child);
                }
            },

            RubyTxtToken::RubyStart => {
                // PositionStartDelimiter なしルビ
                let ruby = parse_ruby(tokens)?;
                tokens = ruby.0;
                let ruby = ruby.1;

                // 空のルビはルビにせず "《》" を入れる
                if ruby.is_empty() {
                    elements.push_str("《》");
                    continue;
                }

                elements.apply_string_buffer();

                // 範囲を探索してルビを振る
                match elements.pop().context("Cannod set ruby to None")? {
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

                    el => bail!("Cannot set ruby {:?} to {:?}", ruby, el),
                }
            }

            RubyTxtToken::RubyEnd => {
                // 対応する '《' があったならここに来ないので '》' を入れる
                tokens = &tokens[1..];
                elements.push_char('》');
            }

            RubyTxtToken::GaijiAnnotationStart => {
                let gaiji = parse_gaiji_annotation(tokens)?;
                tokens = gaiji.0;
                let gaiji = gaiji.1;
                match gaiji {
                    ParsedGaijiAnnotation::String(gaiji) => {
                        elements.push_str(&gaiji);
                    }
                    ParsedGaijiAnnotation::Unknown(description) => {
                        elements.push(BookContentElement::String {
                            value: format!("※［{}］", description),
                            ruby: None,
                        });
                    }
                }
            }

            RubyTxtToken::GaijiAccentDecompositionStart => {
                match parse_gaiji_accent_decomposition(tokens)? {
                    ParsedGaijiAccentDecomposition::NotAccentDecomposition => {
                        tokens = &tokens[1..];
                        elements.push_char('〔');
                    }
                    ParsedGaijiAccentDecomposition::Composed(t, children) => {
                        tokens = t;
                        elements.extend(children);
                    }
                }
            }

            RubyTxtToken::GaijiAccentDecompositionEnd => {
                // 対応するアクセント分解があったならここに来ないので '〕' を入れる
                tokens = &tokens[1..];
                elements.push_char('〕');
            }

            _ => {
                // TODO
                tokens = &tokens[1..];
            }
        }
    }

    ensure!(tokens.is_empty());

    Ok(elements.collect_to_vec())
}

// RubyStart ... RubyEnd
fn parse_ruby<'a>(tokens: &'a [&'a RubyTxtToken]) -> Result<(&'a [&'a RubyTxtToken], String)> {
    ensure!(matches!(tokens.get(0), Some(RubyTxtToken::RubyStart)));
    let mut tokens = &tokens[1..];

    let end_index = {
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
        end_index
    }
    .context("A line ends without '》'")?;

    let child_tokens = &tokens[..end_index];
    tokens = &tokens[(end_index + 1)..];

    let child_elements = parse_block(&child_tokens)?;
    if child_elements.is_empty() {
        return Ok((tokens, "".to_owned()));
    }
    ensure!(
        child_elements.len() == 1,
        "Invalid ruby: {:?}",
        child_elements
    );

    let ruby = match &child_elements[0] {
        BookContentElement::String {
            value: child_value,
            ruby: None,
        } => child_value.clone(),
        el => bail!("Invalid element is found in Ruby: {:?}", el),
    };

    Ok((tokens, ruby))
}

enum ParsedGaijiAnnotation {
    String(String),
    Unknown(String),
}

// GaijiAnnotationStart String AnnotationEnd
fn parse_gaiji_annotation<'a>(
    tokens: &'a [&'a RubyTxtToken],
) -> Result<(&'a [&'a RubyTxtToken], ParsedGaijiAnnotation)> {
    ensure!(matches!(
        tokens.get(0),
        Some(RubyTxtToken::GaijiAnnotationStart)
    ));

    let tokens = &tokens[1..];

    let end_index = {
        let mut end_index = None;
        let mut level = 0;
        for (i, &token) in tokens.iter().enumerate() {
            match token {
                &RubyTxtToken::GaijiAnnotationStart => {
                    level += 1;
                }
                &RubyTxtToken::AnnotationStart => {
                    bail!("Cannot write Annotation in GaijiAnnotation");
                }
                &RubyTxtToken::AnnotationEnd => {
                    if level == 0 {
                        end_index = Some(i);
                        break;
                    }
                    level -= 1;
                }
                &RubyTxtToken::NewLine => break,
                _ => continue,
            }
        }
        end_index
    }
    .context("A line ends without '］'")?;

    let child_tokens = &tokens[..end_index];
    let tokens = &tokens[(end_index + 1)..];

    let child_elements = parse_block(&child_tokens)?;
    ensure!(child_elements.len() == 1);

    let annotation = match &child_elements[0] {
        BookContentElement::String { value, ruby: None } => value,
        t => bail!("Invalid gaiji annotation: {:?}", t),
    };

    // 変体仮名
    static REGEX_HENTAIGANA: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^変体仮名(?P<kana>.).*$").unwrap());

    if let Some(caps) = REGEX_HENTAIGANA.captures(&annotation) {
        let kana = caps.name("kana").unwrap().as_str();
        return Ok((tokens, ParsedGaijiAnnotation::String(kana.to_string())));
    }

    // 外字（第 1 第 2 水準にない漢字：第 3 第 4 水準にある & 特殊な仮名や記号など）
    static REGEX_JIS: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[^、]+、第[3-4]水準?(?P<plane>[0-9]+)-(?P<row>[0-9]+)-(?P<cell>[0-9]+)$")
            .unwrap()
    });

    if let Some(caps) = REGEX_JIS.captures(&annotation) {
        let plane = caps
            .name("plane")
            .unwrap()
            .as_str()
            .parse()
            .context("Invalid plane")?;
        let row = caps
            .name("row")
            .unwrap()
            .as_str()
            .parse()
            .context("Invalid row")?;
        let cell = caps
            .name("cell")
            .unwrap()
            .as_str()
            .parse()
            .context("Invalid cell")?;
        let char = jis_x_0213::JIS_X_0213
            .get(&(plane, row, cell))
            .with_context(|| format!("Unknown JIS code: {}-{}-{}", plane, row, cell))?;

        return Ok((tokens, ParsedGaijiAnnotation::String(char.clone())));
    }

    // 外字（第 1 第 2 水準にない漢字：JIS X 0213 にないが Unicode にある，特殊な仮名や記号など）
    static REGEX_UNICODE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^.+?、U\+(?P<unicode>[0-9A-Fa-f]+)、[0-9]+-[0-9]+$").unwrap());

    if let Some(caps) = REGEX_UNICODE.captures(&annotation) {
        let unicode = caps.name("unicode").unwrap().as_str();
        let unicode = u32::from_str_radix(unicode, 16).context("Invalid unicode")?;
        let char = char::from_u32(unicode).context("Invalid unicode")?;

        return Ok((tokens, ParsedGaijiAnnotation::String(char.to_string())));
    }

    // TODO
    Ok((tokens, ParsedGaijiAnnotation::Unknown(annotation.clone())))
}

enum ParsedDelimiterAndTokens<'a> {
    NotDelimiter,
    Element(&'a [&'a RubyTxtToken], BookContentElement),
}

// PositionStartDelimiter ... (RubyStart ... RubyEnd)
fn parse_delimiter_and_tokens<'a>(
    tokens: &'a [&'a RubyTxtToken],
) -> Result<ParsedDelimiterAndTokens<'a>> {
    ensure!(matches!(
        tokens.get(0),
        Some(RubyTxtToken::PositionStartDelimiter)
    ));

    let mut tokens = &tokens[1..];

    let mut child_tokens = Vec::new();
    while !tokens.is_empty() {
        match tokens[0] {
            RubyTxtToken::RubyStart => {
                let value = parse_block(&child_tokens)?;
                ensure!(
                    value.len() == 1,
                    "Invalid delimiter operands: {:?} ({:?})",
                    value,
                    child_tokens,
                );
                let value = match &value[0] {
                    BookContentElement::String { value, ruby: None } => value,
                    el => bail!("Cannot add ruby to invalid element: {:?}", el),
                };

                let ruby = parse_ruby(&tokens)?;
                tokens = ruby.0;
                let ruby = ruby.1;

                return Ok(ParsedDelimiterAndTokens::Element(
                    tokens,
                    BookContentElement::String {
                        value: value.clone(),
                        ruby: Some(ruby),
                    },
                ));
            }

            RubyTxtToken::NewLine => {
                return Ok(ParsedDelimiterAndTokens::NotDelimiter);
            }

            _ => {
                child_tokens.push(tokens[0]);
                tokens = &tokens[1..];
            }
        }
    }

    Ok(ParsedDelimiterAndTokens::NotDelimiter)
}

enum ParsedGaijiAccentDecomposition<'a> {
    NotAccentDecomposition,
    Composed(&'a [&'a RubyTxtToken], Vec<BookContentElement>),
}

// GaijiAccentDecompositionStart String GaijiAccentDecompositionEnd
fn parse_gaiji_accent_decomposition<'a>(
    tokens: &'a [&'a RubyTxtToken],
) -> Result<ParsedGaijiAccentDecomposition<'a>> {
    ensure!(matches!(
        tokens.get(0),
        Some(RubyTxtToken::GaijiAccentDecompositionStart)
    ));

    let tokens = &tokens[1..];

    let mut processed_tokens = Vec::new();
    let mut composed = false;

    let end_index = {
        let mut end_index = None;
        let mut level = 0;
        for (i, &token) in tokens.iter().enumerate() {
            match token {
                RubyTxtToken::GaijiAccentDecompositionStart => {
                    level += 1;
                }

                RubyTxtToken::GaijiAccentDecompositionEnd => {
                    if level == 0 {
                        end_index = Some(i);
                        break;
                    }
                    level -= 1;
                }

                RubyTxtToken::String(value) => {
                    if level == 0 {
                        let new_value = compose_accent(&value);
                        if value != &new_value {
                            composed = true;
                            processed_tokens.push(RubyTxtToken::String(new_value));
                            continue;
                        }
                    }
                }

                _ => {}
            }

            processed_tokens.push(token.clone());
        }
        end_index
    };

    let end_index = match end_index {
        Some(end_index) => end_index,
        None => return Ok(ParsedGaijiAccentDecomposition::NotAccentDecomposition),
    };

    if !composed {
        return Ok(ParsedGaijiAccentDecomposition::NotAccentDecomposition);
    }

    let processed_tokens = processed_tokens.iter().map(|t| t).collect::<Vec<_>>();
    let child_elements = parse_block(&processed_tokens)?;

    Ok(ParsedGaijiAccentDecomposition::Composed(
        &tokens[(end_index + 1)..],
        child_elements,
    ))
}
