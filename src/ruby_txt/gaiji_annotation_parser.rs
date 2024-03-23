use anyhow::{bail, ensure, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    jis_x_0213,
    ruby_txt::{
        block_parser::parse_block, parser_helper::ParsedRubyTxtElement, tokenizer::RubyTxtToken,
    },
};

pub(super) enum ParsedGaijiAnnotation {
    String(String),
    Unknown(String),
}

// GaijiAnnotationStart String AnnotationEnd
pub(super) fn parse_gaiji_annotation<'a>(
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
    ensure!(
        child_elements.len() == 1,
        "Invalid gaiji annotation: {:?}",
        child_elements
    );

    let annotation = match &child_elements[0] {
        ParsedRubyTxtElement::String { value } => value,
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
        Regex::new(r"^[^、]+、(第[3-4]水準)?(?P<plane>[0-9]+)-(?P<row>[0-9]+)-(?P<cell>[0-9]+)$")
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
        let char = jis_x_0213::JIS_X_0213.get(&(plane, row, cell));

        if let Some(char) = char {
            return Ok((tokens, ParsedGaijiAnnotation::String(char.clone())));
        }
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
