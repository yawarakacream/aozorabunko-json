use anyhow::{bail, Result};

// 青空文庫に向けた文字種別
// 仝々〆〇ヶ は漢字扱い (https://www.aozora.gr.jp/annotation/etc.html#ruby)
#[derive(Debug, PartialEq, Eq)]
pub enum CharType {
    LatinAlphabet,
    Hiragana,
    Katakana,
    Kanji,
    Other,
}

impl CharType {
    pub fn from(c: char) -> Self {
        let u = c as u32;

        if 0x0041 <= u && u <= 0x005a || 0x0061 <= u && u <= 0x007a {
            // 小文字・大文字
            Self::LatinAlphabet
        } else if 0x00c0 <= u && u <= 0x00ff && u != 0x00d7 && u != 0x00f7 {
            // アクセント記号付き
            Self::LatinAlphabet
        } else if 0x3040 <= u && u <= 0x309f {
            Self::Hiragana
        } else if 0x30a0 <= u && u <= 0x30ff {
            Self::Katakana
        } else if 0x3400 <= u && u <= 0x4dbf
            || 0x4e00 <= u && u <= 0x9fff
            || 0xf900 <= u && u <= 0xfaff
            || c == '仝'
            || c == '々'
            || c == '〆'
            || c == '〇'
            || c == 'ヶ'
        {
            Self::Kanji
        } else {
            Self::Other
        }
    }
}

pub fn parse_number(s: &str) -> Result<usize> {
    let mut ret = 0;
    for c in s.chars() {
        let zero = match c {
            '0'..='9' => '0',
            '０'..='９' => '０',
            _ => bail!("Failed to parse {:?}", s),
        } as usize;

        let d = (c as usize) - zero;

        ret *= 10;
        ret += d;
    }
    Ok(ret)
}
