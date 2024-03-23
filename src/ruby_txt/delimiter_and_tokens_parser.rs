use anyhow::{ensure, Result};

use crate::{
    ruby_txt::parser_helper::ParsedRubyTxtElement,
    ruby_txt::{block_parser::parse_block, ruby_parser::parse_ruby, tokenizer::RubyTxtToken},
};

pub(super) enum ParsedDelimiterAndTokens<'a> {
    NotDelimiter,
    Element(&'a [&'a RubyTxtToken], Vec<ParsedRubyTxtElement>),
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
                child_elements.insert(0, ParsedRubyTxtElement::RubyStart { value: ruby });
                child_elements.push(ParsedRubyTxtElement::RubyEnd);

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
