use anyhow::{ensure, Context, Result};

use crate::{
    ruby_txt::{
        annotation_parser::parse_annotation,
        delimiter_and_tokens_parser::{parse_delimiter_and_tokens, ParsedDelimiterAndTokens},
        gaiji_accent_decomposition_parser::{
            parse_gaiji_accent_decomposition, ParsedGaijiAccentDecomposition,
        },
        gaiji_annotation_parser::{parse_gaiji_annotation, ParsedGaijiAnnotation},
        parser_helper::{ParsedRubyTxtElement, ParsedRubyTxtElementList},
        ruby_parser::parse_ruby,
        tokenizer::RubyTxtToken,
    },
    utility::CharType,
};

pub(super) fn parse_block<'a>(tokens: &'a [&'a RubyTxtToken]) -> Result<Vec<ParsedRubyTxtElement>> {
    let mut tokens = tokens;
    let mut elements = ParsedRubyTxtElementList::new();

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
                elements.push(ParsedRubyTxtElement::NewLine);
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
                        ParsedRubyTxtElement::String { value } => {
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
                                elements.push(ParsedRubyTxtElement::String {
                                    value: value_chars[..ruby_start_index].iter().collect(),
                                });
                            }
                            elements.push(ParsedRubyTxtElement::RubyStart { value: ruby });
                            elements.push(ParsedRubyTxtElement::String {
                                value: value_chars[ruby_start_index..].iter().collect(),
                            });
                            elements.push(ParsedRubyTxtElement::RubyEnd);
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
                        elements.push(ParsedRubyTxtElement::String {
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
