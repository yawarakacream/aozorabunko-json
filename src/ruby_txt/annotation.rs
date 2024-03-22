use anyhow::{bail, ensure, Context, Ok, Result};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    book_content::BookContentElement,
    ruby_txt::{ruby_txt_parser::parse_block, ruby_txt_tokenizer::RubyTxtToken},
    utility::parse_number,
};

// AnnotationStart ... AnnotationEnd
pub(super) fn parse_annotation<'a>(
    tokens: &'a [&'a RubyTxtToken],
) -> Result<(&'a [&'a RubyTxtToken], BookContentElement)> {
    ensure!(matches!(tokens.get(0), Some(RubyTxtToken::AnnotationStart)));
    let tokens = &tokens[1..];

    let end_index = {
        let mut end_index = None;
        let mut level = 0;
        for (i, &token) in tokens.iter().enumerate() {
            match token {
                &RubyTxtToken::AnnotationStart | &RubyTxtToken::GaijiAnnotationStart => {
                    level += 1;
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

    let args = &tokens[..end_index];
    let tokens = &tokens[(end_index + 1)..];

    let args = parse_block(args)?;
    let annotation = match args.len() {
        // 空の annotation は "［＃］：入力者注　主に外字の説明や、傍点の位置の指定" のように使われることがある
        0 => BookContentElement::String {
            value: "［＃］".to_owned(),
        },

        1 => {
            let arg = &args[0];
            let arg = match &arg {
                &BookContentElement::String { value } => value,
                _ => bail!("Unknown annotation: {:?}", arg),
            };

            // 正規表現を使いつつのうまい match の書き方がわからない
            // match が使えないなら早期リターンしたいので苦肉の策
            let ret: Result<BookContentElement> = (|| {
                if arg == "改丁" {
                    return Ok(BookContentElement::KaichoAttention);
                }

                if arg == "改ページ" {
                    return Ok(BookContentElement::KaipageAttention);
                }

                if arg == "改見開き" {
                    return Ok(BookContentElement::KaimihirakiAttention);
                }

                if arg == "改段" {
                    return Ok(BookContentElement::KaidanAttention);
                }

                static REGEX_JISAGE: Lazy<Regex> =
                    Lazy::new(|| Regex::new(r"^(?P<level>[０-９]+)字下げ$").unwrap());
                if let Some(caps) = REGEX_JISAGE.captures(&arg) {
                    let level = parse_number(caps.name("level").unwrap().as_str())
                        .with_context(|| format!("Failed to parse {:?}", arg))?;
                    return Ok(BookContentElement::JisageAnnotation { level });
                }

                static REGEX_JISAGE_START: Lazy<Regex> =
                    Lazy::new(|| Regex::new(r"^ここから(?P<level>[０-９]+)字下げ$").unwrap());
                if let Some(caps) = REGEX_JISAGE_START.captures(&arg) {
                    let level = parse_number(caps.name("level").unwrap().as_str())
                        .with_context(|| format!("Failed to parse {:?}", arg))?;
                    return Ok(BookContentElement::JisageStartAnnotation { level });
                }

                static REGEX_JISAGE_WITH_ORIKAESHI_START: Lazy<Regex> = Lazy::new(|| {
                    Regex::new(r"^ここから(?P<level0>[０-９]+)字下げ、折り返して(?P<level1>[０-９]+)字下げ$").unwrap()
                });
                if let Some(caps) = REGEX_JISAGE_WITH_ORIKAESHI_START.captures(&arg) {
                    let level0 = parse_number(caps.name("level0").unwrap().as_str())
                        .with_context(|| format!("Failed to parse {:?}", arg))?;
                    let level1 = parse_number(caps.name("level1").unwrap().as_str())
                        .with_context(|| format!("Failed to parse {:?}", arg))?;
                    return Ok(BookContentElement::JisageWithOrikaeshiStartAnnotation {
                        level0,
                        level1,
                    });
                }

                static REGEX_JISAGE_AFTER_TENTSUKI_START: Lazy<Regex> = Lazy::new(|| {
                    Regex::new(r"^ここから改行天付き、折り返して(?P<level>[０-９]+)字下げ$")
                        .unwrap()
                });
                if let Some(caps) = REGEX_JISAGE_AFTER_TENTSUKI_START.captures(&arg) {
                    let level = parse_number(caps.name("level").unwrap().as_str())
                        .with_context(|| format!("Failed to parse {:?}", arg))?;
                    return Ok(BookContentElement::JisageAfterTentsukiStartAnnotation { level });
                }

                if arg == "ここで字下げ終わり" {
                    return Ok(BookContentElement::JisageEndAnnotation);
                }

                if arg == "地付き" {
                    return Ok(BookContentElement::JitsukiAnnotation);
                }

                if arg == "ここから地付き" {
                    return Ok(BookContentElement::JitsukiStartAnnotation);
                }

                if arg == "ここで地付き終わり" {
                    return Ok(BookContentElement::JitsukiEndAnnotation);
                }

                static REGEX_JIYOSE: Lazy<Regex> =
                    Lazy::new(|| Regex::new(r"^地から(?P<level>[０-９]+)字上げ$").unwrap());
                if let Some(caps) = REGEX_JIYOSE.captures(&arg) {
                    let level = parse_number(caps.name("level").unwrap().as_str())
                        .with_context(|| format!("Failed to parse {:?}", arg))?;
                    return Ok(BookContentElement::JiyoseAnnotation { level });
                }

                static REGEX_JIYOSE_START: Lazy<Regex> =
                    Lazy::new(|| Regex::new(r"^ここから地から(?P<level>[０-９]+)字上げ$").unwrap());
                if let Some(caps) = REGEX_JIYOSE_START.captures(&arg) {
                    let level = parse_number(caps.name("level").unwrap().as_str())
                        .with_context(|| format!("Failed to parse {:?}", arg))?;
                    return Ok(BookContentElement::JiyoseStartAnnotation { level });
                }

                if arg == "ここで字上げ終わり" {
                    return Ok(BookContentElement::JiyoseEndAnnotation);
                }

                if arg == "ページの左右中央" {
                    return Ok(BookContentElement::PageCenterAnnotation);
                }

                Ok(BookContentElement::String {
                    value: format!("[Unknown Annotation]({})", arg),
                })
            })();
            ret?
        }

        _ => {
            // TODO
            BookContentElement::String {
                value: "[Unknown Annotation]".to_owned(),
            }
        }
    };

    Ok((tokens, annotation))
}
