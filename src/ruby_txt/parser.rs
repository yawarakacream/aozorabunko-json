use anyhow::{ensure, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::ruby_txt::{
    block_parser::parse_block,
    parser_helper::{ParsedRubyTxt, ParsedRubyTxtElement},
    tokenizer::RubyTxtToken,
};

// 構文解析
pub fn parse_ruby_txt(tokens: &[RubyTxtToken]) -> Result<ParsedRubyTxt> {
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
            if !matches!(last, ParsedRubyTxtElement::NewLine) {
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
                if !matches!(last, ParsedRubyTxtElement::KaipageAttention) {
                    elements.push(ParsedRubyTxtElement::KaipageAttention);
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
            if !matches!(last, ParsedRubyTxtElement::NewLine) {
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
            if !matches!(last, ParsedRubyTxtElement::NewLine) {
                break;
            }
            elements.pop();
        }
        ensure!(!elements.is_empty(), "Footer is empty");

        elements
    };

    Ok(ParsedRubyTxt {
        header,
        body,
        footer,
    })
}
