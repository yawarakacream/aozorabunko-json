use anyhow::{bail, ensure, Result};

use crate::{
    book_content::BookContentElement,
    ruby_txt::{
        ruby::parse_ruby,
        ruby_txt_parser::{parse_block, RubyTxtToken},
    },
};

pub(super) enum ParsedDelimiterAndTokens<'a> {
    NotDelimiter,
    Element(&'a [&'a RubyTxtToken], BookContentElement),
}

// PositionStartDelimiter ... (RubyStart ... RubyEnd)
pub(super) fn parse_delimiter_and_tokens<'a>(
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
