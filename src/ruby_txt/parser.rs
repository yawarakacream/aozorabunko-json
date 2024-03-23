use anyhow::{ensure, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::ruby_txt::{
    block_parser::parse_block,
    tokenizer::RubyTxtToken,
    utility::{
        BouDecorationSide, BouDecorationStyle, MidashiLevel, MidashiStyle, StringDecorationStyle,
    },
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRubyTxt {
    pub header: Vec<ParsedRubyTxtElement>,
    pub body: Vec<ParsedRubyTxtElement>,
    pub footer: Vec<ParsedRubyTxtElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum ParsedRubyTxtElement {
    String {
        value: String,
    },
    NewLine,
    UnknownAnnotation {
        // 非空
        args: Vec<ParsedRubyTxtElement>,
    },

    // ｜
    PositionMarker,

    // 《○○》
    Ruby {
        // 非空
        value: Vec<ParsedRubyTxtElement>,
    },

    KaichoAttention,      // ［＃改丁］
    KaipageAttention,     // ［＃改ページ］
    KaimihirakiAttention, // ［＃改見開き］
    KaidanAttention,      // ［＃改段］

    // ［＃○字下げ］ => { level: ○ }
    JisageAnnotation {
        level: usize,
    },
    // ［＃ここから○字下げ］ => { level: ○ }
    JisageStartAnnotation {
        level: usize,
    },
    // ［＃ここから○字下げ、折り返して●字下げ］ => { level0: ○, level1: ● }
    JisageWithOrikaeshiStartAnnotation {
        level0: usize,
        level1: usize,
    },
    // ［＃ここから改行天付き、折り返して○字下げ］ => { level: ○ }
    JisageAfterTentsukiStartAnnotation {
        level: usize,
    },
    // ［＃ここで字下げ終わり］
    JisageEndAnnotation,

    // ［＃地付き］
    JitsukiAnnotation,
    // ［＃ここから地付き］
    JitsukiStartAnnotation,
    // ［＃ここで地付き終わり］
    JitsukiEndAnnotation,

    // ［＃地から○字上げ］
    JiyoseAnnotation {
        level: usize,
    },
    // ［＃ここから地から○字上げ］
    JiyoseStartAnnotation {
        level: usize,
    },
    // ［＃ここで字上げ終わり］
    JiyoseEndAnnotation,

    // ［＃ページの左右中央］
    PageCenterAnnotation,

    // 見出し
    Midashi {
        value: String,
        level: MidashiLevel,
        style: MidashiStyle,
    },
    MidashiStart {
        level: MidashiLevel,
        style: MidashiStyle,
    },
    MidashiEnd {
        level: MidashiLevel,
        style: MidashiStyle,
    },

    // 返り点
    Kaeriten {
        // 0:［＃一］, 1:［＃二］, 2:［＃三］, 3:［＃四］
        ichini: Option<usize>,
        // 0:［＃上］, 1:［＃中］, 2:［＃下］
        jouge: Option<usize>,
        // 0:［＃甲］, 1:［＃乙］, 2:［＃丙］, 3:［＃丁］
        kouotsu: Option<usize>,
        // false: なし, true:［＃レ］
        re: bool,
    },
    // ［＃（○○）］
    KuntenOkurigana {
        value: String,
    },

    // 傍点・傍線
    BouDecoration {
        target: Vec<ParsedRubyTxtElement>,
        side: BouDecorationSide,
        style: BouDecorationStyle,
    },
    BouDecorationStart {
        side: BouDecorationSide,
        style: BouDecorationStyle,
    },
    BouDecorationEnd {
        side: BouDecorationSide,
        style: BouDecorationStyle,
    },

    // 太字・斜体
    StringDecoration {
        target: Vec<ParsedRubyTxtElement>,
        style: StringDecorationStyle,
    },
    StringDecorationStart {
        style: StringDecorationStyle,
    },
    StringDecorationEnd {
        style: StringDecorationStyle,
    },

    // ［＃○○（●●.png）入る］
    Image {
        path: String,
        alt: String,
    },
    // ［＃「○○」はキャプション］
    Caption {
        value: Vec<ParsedRubyTxtElement>,
    },
    // ［＃キャプション］
    CaptionStart,
    // ［＃キャプション終わり］
    CaptionEnd,

    // ［＃割り注］
    WarichuStart,
    // ［＃割り注終わり］
    WarichuEnd,
}

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
