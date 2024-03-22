use anyhow::{ensure, Result};

use crate::{
    book_content::BookContentElement,
    ruby_txt::ruby_txt_parser::{parse_block, RubyTxtToken},
};

pub(super) enum ParsedGaijiAccentDecomposition<'a> {
    NotAccentDecomposition,
    Composed(&'a [&'a RubyTxtToken], Vec<BookContentElement>),
}

// GaijiAccentDecompositionStart String GaijiAccentDecompositionEnd
pub(super) fn parse_gaiji_accent_decomposition<'a>(
    tokens: &'a [&'a RubyTxtToken],
) -> Result<ParsedGaijiAccentDecomposition<'a>> {
    ensure!(matches!(
        tokens.get(0),
        Some(RubyTxtToken::GaijiAccentDecompositionStart)
    ));
    let tokens = &tokens[1..];

    let mut processed_tokens = Vec::new();
    let mut composed = false;

    let end_index = {
        let mut end_index = None;
        let mut level = 0;
        for (i, &token) in tokens.iter().enumerate() {
            match token {
                RubyTxtToken::GaijiAccentDecompositionStart => {
                    level += 1;
                }

                RubyTxtToken::GaijiAccentDecompositionEnd => {
                    if level == 0 {
                        end_index = Some(i);
                        break;
                    }
                    level -= 1;
                }

                RubyTxtToken::String(value) => {
                    if level == 0 {
                        let new_value = compose_accent(&value);
                        if value != &new_value {
                            composed = true;
                            processed_tokens.push(RubyTxtToken::String(new_value));
                            continue;
                        }
                    }
                }

                _ => {}
            }

            processed_tokens.push(token.clone());
        }
        end_index
    };

    let end_index = match end_index {
        Some(end_index) => end_index,
        None => return Ok(ParsedGaijiAccentDecomposition::NotAccentDecomposition),
    };

    if !composed {
        return Ok(ParsedGaijiAccentDecomposition::NotAccentDecomposition);
    }

    let processed_tokens = processed_tokens.iter().map(|t| t).collect::<Vec<_>>();
    let child_elements = parse_block(&processed_tokens)?;

    Ok(ParsedGaijiAccentDecomposition::Composed(
        &tokens[(end_index + 1)..],
        child_elements,
    ))
}

// https://www.aozora.gr.jp/accent_separation.html
fn compose_accent(s: &str) -> String {
    let mut ret = String::new();

    let s: Vec<_> = s.chars().collect();
    let mut i = 0;
    while i < s.len() {
        let c0 = s[i];

        if let Some(&c1) = s.get(i + 1) {
            let c = match (c0, c1) {
                ('a', '`') => 'à',
                ('a', '\'') => 'á',
                ('a', '^') => 'â',
                ('a', '~') => 'ã',
                ('a', ':') => 'ä',
                ('a', '&') => 'å',
                ('a', '_') => 'ā',

                ('c', ',') => 'ç',
                ('c', '\'') => 'ć',
                ('c', '^') => 'ĉ',

                ('d', '/') => 'đ',

                ('e', '`') => 'è',
                ('e', '\'') => 'é',
                ('e', '^') => 'ê',
                ('e', ':') => 'ë',
                ('e', '_') => 'ē',
                ('e', '~') => 'ẽ',

                ('g', '^') => 'ĝ',

                ('h', '^') => 'ĥ',
                ('h', '/') => 'ħ',

                ('i', '`') => 'ì',
                ('i', '\'') => 'í',
                ('i', '^') => 'î',
                ('i', ':') => 'ï',
                ('i', '_') => 'ī',
                ('i', '/') => 'ɨ',
                ('i', '~') => 'ĩ',

                ('j', '^') => 'ĵ',

                ('l', '/') => 'ł',
                ('l', '\'') => 'ĺ',

                ('m', '\'') => 'ḿ',

                ('n', '`') => 'ǹ',
                ('n', '~') => 'ñ',
                ('n', '\'') => 'ń',

                ('o', '`') => 'ò',
                ('o', '\'') => 'ó',
                ('o', '^') => 'ô',
                ('o', '~') => 'õ',
                ('o', ':') => 'ö',
                ('o', '/') => 'ø',
                ('o', '_') => 'ō',

                ('r', '\'') => 'ŕ',

                ('s', '\'') => 'ś',
                ('s', ',') => 'ş',
                ('s', '^') => 'ŝ',

                ('t', ',') => 'ţ',

                ('u', '`') => 'ù',
                ('u', '\'') => 'ú',
                ('u', '^') => 'û',
                ('u', ':') => 'ü',
                ('u', '_') => 'ū',
                ('u', '&') => 'ů',
                ('u', '~') => 'ũ',

                ('y', '\'') => 'ý',
                ('y', ':') => 'ÿ',

                ('z', '\'') => 'ź',

                ('A', '`') => 'À',
                ('A', '\'') => 'Á',
                ('A', '^') => 'Â',
                ('A', '~') => 'Ã',
                ('A', ':') => 'Ä',
                ('A', '&') => 'Å',
                ('A', '_') => 'Ā',

                ('C', ',') => 'Ç',
                ('C', '\'') => 'Ć',
                ('C', '^') => 'Ĉ',

                ('D', '/') => 'Đ',

                ('E', '`') => 'È',
                ('E', '\'') => 'É',
                ('E', '^') => 'Ê',
                ('E', ':') => 'Ë',
                ('E', '_') => 'Ē',
                ('E', '~') => 'Ẽ',

                ('G', '^') => 'Ĝ',

                ('H', '^') => 'Ĥ',

                ('I', '`') => 'Ì',
                ('I', '\'') => 'Í',
                ('I', '^') => 'Î',
                ('I', ':') => 'Ï',
                ('I', '_') => 'Ī',
                ('I', '~') => 'Ĩ',

                ('J', '^') => 'Ĵ',

                ('L', '/') => 'Ł',
                ('L', '\'') => 'Ĺ',

                ('M', '\'') => 'Ḿ',

                ('N', '`') => 'Ǹ',
                ('N', '~') => 'Ñ',
                ('N', '\'') => 'Ń',

                ('O', '`') => 'Ò',
                ('O', '\'') => 'Ó',
                ('O', '^') => 'Ô',
                ('O', '~') => 'Õ',
                ('O', ':') => 'Ö',
                ('O', '/') => 'Ø',
                ('O', '_') => 'Ō',

                ('R', '\'') => 'Ŕ',

                ('S', '\'') => 'Ś',
                ('S', ',') => 'Ş',
                ('S', '^') => 'Ŝ',

                ('T', ',') => 'Ţ',

                ('U', '`') => 'Ù',
                ('U', '\'') => 'Ú',
                ('U', '^') => 'Û',
                ('U', ':') => 'Ü',
                ('U', '_') => 'Ū',
                ('U', '&') => 'Ů',
                ('U', '~') => 'Ũ',

                ('Y', '\'') => 'Ý',

                ('Z', '\'') => 'Ź',

                ('s', '&') => 'ß',

                _ => c0,
            };

            if c != c0 {
                i += 2;
                ret.push(c);
                continue;
            }

            if let Some(&c2) = s.get(i + 2) {
                let c = match (c0, c1, c2) {
                    ('a', 'e', '&') => 'æ',
                    ('A', 'E', '&') => 'Æ',
                    ('o', 'e', '&') => 'œ',
                    ('O', 'E', '&') => 'Œ',

                    _ => c0,
                };

                if c != c0 {
                    i += 3;
                    ret.push(c);
                    continue;
                }
            }
        }

        i += 1;
        ret.push(c0);
    }

    ret
}
