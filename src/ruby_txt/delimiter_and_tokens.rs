use anyhow::{ensure, Result};

use crate::{
    book_content::BookContentElement,
    ruby_txt::{
        ruby::parse_ruby,
        ruby_txt_parser::{parse_block, RubyTxtToken},
    },
};

pub(super) enum ParsedDelimiterAndTokens<'a> {
    NotDelimiter,
    Element(&'a [&'a RubyTxtToken], Vec<BookContentElement>),
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
                let ruby = parse_ruby(&tokens)?;
                tokens = ruby.0;
                let ruby = ruby.1;

                let mut child_elements = parse_block(&child_tokens)?;
                child_elements.insert(0, BookContentElement::RubyStart { value: ruby });
                child_elements.push(BookContentElement::RubyEnd);

                return Ok(ParsedDelimiterAndTokens::Element(tokens, child_elements));
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
