use anyhow::Result;
use serde::{Deserialize, Serialize};

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
