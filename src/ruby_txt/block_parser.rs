use anyhow::{ensure, Result};

use crate::ruby_txt::{
    annotation_parser::parse_annotation,
    gaiji_accent_decomposition_parser::{
        parse_gaiji_accent_decomposition, ParsedGaijiAccentDecomposition,
    },
    gaiji_annotation_parser::{parse_gaiji_annotation, ParsedGaijiAnnotation},
    parser::ParsedRubyTxtElement,
    parser_helper::ParsedRubyTxtElementList,
    ruby_parser::parse_ruby,
    tokenizer::RubyTxtToken,
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

            RubyTxtToken::PositionMarker => {
                tokens = &tokens[1..];
                elements.push(ParsedRubyTxtElement::PositionMarker);
            }

            RubyTxtToken::RubyStart => {
                // PositionStartDelimiter なしルビ
                let ruby = parse_ruby(tokens)?;

                tokens = ruby.0;

                if ruby.1.is_empty() {
                    // 空のルビはルビにせず "《》" を入れる
                    elements.push_str("《》");
                } else {
                    elements.push(ParsedRubyTxtElement::Ruby { value: ruby.1 });
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
