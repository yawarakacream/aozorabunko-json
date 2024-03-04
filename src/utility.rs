use std::io::{self, Read};

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use zip::{read::ZipFile, ZipArchive};

#[derive(Debug, PartialEq, Eq, Serialize)]
pub enum Date {
    Y {
        year: usize,
    },
    YM {
        year: usize,
        month: usize,
    },
    YMD {
        year: usize,
        month: usize,
        date: usize,
    },
}

impl Date {
    pub fn parse(date: &str, delimiter: &[char]) -> Result<Date> {
        let ymd: Vec<&str> = date.split(delimiter).collect();

        let year = ymd[0]
            .parse()
            .with_context(|| format!("Invalid year: {:?}", ymd[0]))?;

        if ymd.len() == 1 {
            return Ok(Date::Y { year });
        }

        let month = ymd[1]
            .parse()
            .with_context(|| format!("Invalid month: {:?}", ymd[1]))?;

        if ymd.len() == 2 {
            return Ok(Date::YM { year, month });
        }

        let date = ymd[2]
            .parse()
            .with_context(|| format!("Invalid date: {:?}", ymd[2]))?;

        if ymd.len() == 3 {
            return Ok(Date::YMD { year, month, date });
        }

        Err(anyhow!("Invalid date: {:?}", date))
    }

    pub fn is_equivalent_or_later(&self, other: &Self) -> bool {
        match (&self, &other) {
            (
                Date::YMD { year, month, date },
                Date::YMD {
                    year: other_year,
                    month: other_month,
                    date: other_date,
                },
            ) => {
                if year < other_year {
                    return false;
                }
                if year > other_year {
                    return true;
                }
                if month < other_month {
                    return false;
                }
                if month > other_month {
                    return true;
                }
                if date < other_date {
                    return false;
                }
                if date > other_date {
                    return false;
                }
                true
            }
            _ => unimplemented!(),
        }
    }
}

pub struct ZipReader<R> {
    archive: ZipArchive<R>,
}

impl<R: Read + io::Seek> ZipReader<R> {
    pub fn new(reader: R) -> Result<ZipReader<R>> {
        let archive = ZipArchive::new(reader).context("Failed to open")?;
        Ok(ZipReader { archive })
    }

    pub fn len(self: &Self) -> usize {
        self.archive.len()
    }

    pub fn get_by_path(&mut self, path: &str) -> Result<ZipEntry> {
        self.archive
            .by_name(path)
            .with_context(|| format!("Failed to open {}", path))
            .map(|file| ZipEntry { file })
    }

    pub fn get_by_index(&mut self, index: usize) -> Result<ZipEntry> {
        self.archive
            .by_index(index)
            .with_context(|| format!("Failed to open at {}", index))
            .map(|file| ZipEntry { file })
    }
}

pub struct ZipEntry<'a> {
    file: ZipFile<'a>,
}

impl ZipEntry<'_> {
    pub fn name(self: &Self) -> &str {
        self.file.name()
    }

    pub fn as_bytes(self: &mut Self) -> Result<Vec<u8>> {
        let mut data = Vec::<u8>::new();
        self.file
            .read_to_end(&mut data)
            .with_context(|| format!("Failed to read {}", self.name()))?;

        Ok(data)
    }

    pub fn as_string(self: &mut Self) -> Result<String> {
        let mut data = String::new();
        self.file
            .read_to_string(&mut data)
            .with_context(|| format!("Failed to read {}", self.name()))?;

        Ok(data)
    }
}

pub fn trim_empty_lines(vec: &mut Vec<&str>) {
    let mut i = 0;
    while i < vec.len() && vec[i].is_empty() {
        i += 1;
    }
    vec.drain(..i);

    let mut j = vec.len();
    while 1 <= j && vec[j - 1].is_empty() {
        j -= 1;
    }
    vec.drain(j..);
}

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
        } else if 0x4e00 <= u && u <= 0x9fff
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
