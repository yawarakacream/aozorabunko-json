use anyhow::{ensure, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    book_content::{
        BookContent, BookContentElement, BookContentElementList, BookContentOriginalDataType,
    },
    ruby_txt::{
        annotation::parse_annotation,
        delimiter_and_tokens::{parse_delimiter_and_tokens, ParsedDelimiterAndTokens},
        gaiji_accent_decomposition::{
            parse_gaiji_accent_decomposition, ParsedGaijiAccentDecomposition,
        },
        gaiji_annotation::{parse_gaiji_annotation, ParsedGaijiAnnotation},
        ruby::parse_ruby,
        ruby_txt_tokenizer::RubyTxtToken,
    },
    utility::CharType,
};

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

        elements
    };

    // 冒頭から本文の間の空白行を飛ばす
    while tokens.get(0).context("Body is empty")? == &RubyTxtToken::NewLine {
        tokens = &tokens[1..];
    }

    let body = {
        // "底本："
        static REGEX_FOOTER_CHECKER: Lazy<Regex> = Lazy::new(|| Regex::new(r"^底本[：:]").unwrap());

        let mut blocks = vec![vec![]];
        loop {
            let token = tokens.get(0).context("Failed to load body")?;

            if let RubyTxtToken::String(string) = token {
                if REGEX_FOOTER_CHECKER.is_match(&string) {
                    break;
                }
            }

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

        elements
    };

    Ok(BookContent {
        original_data_type: BookContentOriginalDataType::RubyTxt,
        header,
        body,
        footer,
    })
}

pub(super) fn parse_block<'a>(tokens: &'a [&'a RubyTxtToken]) -> Result<Vec<BookContentElement>> {
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
                ParsedDelimiterAndTokens::Element(t, children) => {
                    tokens = t;
                    elements.extend(children);
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
                let mut passed = Vec::new();
                loop {
                    match elements.pop().context("Cannod find String to set ruby")? {
                        BookContentElement::String { value } => {
                            ensure!(!value.is_empty(), "Cannot set ruby to empty String");

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
                                });
                            }
                            elements.push(BookContentElement::RubyStart { value: ruby });
                            elements.push(BookContentElement::String {
                                value: value_chars[ruby_start_index..].iter().collect(),
                            });
                            elements.push(BookContentElement::RubyEnd);
                            while let Some(el) = passed.pop() {
                                elements.push(el);
                            }

                            break;
                        }

                        el => passed.push(el),
                    }
                }
            }

            RubyTxtToken::RubyEnd => {
                // 対応する '《' があったならここに来ないので '》' を入れる
                tokens = &tokens[1..];
                elements.push_char('》');
            }

            RubyTxtToken::AnnotationStart => {
                let parsed = parse_annotation(tokens)?;
                tokens = parsed.0;
                if let Some(el) = parsed.1 {
                    elements.push(el);
                }
            }

            RubyTxtToken::AnnotationEnd => {
                // 対応する annotation があったならここに来ないので '］' を入れる
                tokens = &tokens[1..];
                elements.push_char('］');
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
                        // TODO
                        elements.push(BookContentElement::String {
                            value: format!("※［{}］", description),
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
        }
    }

    ensure!(tokens.is_empty());

    Ok(elements.collect_to_vec())
}
