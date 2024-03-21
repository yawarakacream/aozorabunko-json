use anyhow::{bail, ensure, Context, Result};

use crate::{
    book_content::BookContentElement,
    ruby_txt::ruby_txt_parser::{parse_block, RubyTxtToken},
};

// RubyStart ... RubyEnd
pub(super) fn parse_ruby<'a>(
    tokens: &'a [&'a RubyTxtToken],
) -> Result<(&'a [&'a RubyTxtToken], String)> {
    ensure!(matches!(tokens.get(0), Some(RubyTxtToken::RubyStart)));
    let mut tokens = &tokens[1..];

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
    tokens = &tokens[(end_index + 1)..];

    let child_elements = parse_block(&child_tokens)?;
    if child_elements.is_empty() {
        return Ok((tokens, "".to_owned()));
    }
    ensure!(
        child_elements.len() == 1,
        "Invalid ruby: {:?}",
        child_elements
    );

    let ruby = match &child_elements[0] {
        BookContentElement::String {
            value: child_value,
            ruby: None,
        } => child_value.clone(),
        el => bail!("Invalid element is found in Ruby: {:?}", el),
    };

    Ok((tokens, ruby))
}
