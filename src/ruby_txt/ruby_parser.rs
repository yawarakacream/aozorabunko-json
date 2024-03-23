use anyhow::{ensure, Context, Result};

use crate::{
    ruby_txt::parser::ParsedRubyTxtElement,
    ruby_txt::{block_parser::parse_block, tokenizer::RubyTxtToken},
};

// RubyStart ... RubyEnd
pub(super) fn parse_ruby<'a>(
    tokens: &'a [&'a RubyTxtToken],
) -> Result<(&'a [&'a RubyTxtToken], Vec<ParsedRubyTxtElement>)> {
    ensure!(matches!(tokens.get(0), Some(RubyTxtToken::RubyStart)));
    let tokens = &tokens[1..];

    let end_index = {
        let mut end_index = None;
        for (i, &token) in tokens.iter().enumerate() {
            match token {
                &RubyTxtToken::RubyEnd => {
                    end_index = Some(i);
                    break;
                }
                &RubyTxtToken::NewLine => break,
                _ => continue,
            }
        }
        end_index
    }
    .context("A line ends without '》'")?;

    let child_tokens = &tokens[..end_index];
    let tokens = &tokens[(end_index + 1)..];

    let child_elements = parse_block(&child_tokens)?;
    Ok((tokens, child_elements))
}
