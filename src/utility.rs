use anyhow::{bail, Context, Result};
use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Date {
    pub year: usize,
    pub month: Option<usize>,
    pub date: Option<usize>,
}

impl Date {
    pub fn parse(date: &str, delimiter: &[char]) -> Result<Date> {
        let ymd: Vec<&str> = date.split(delimiter).collect();

        if 3 < ymd.len() {
            bail!("Invalid date: {:?}", date);
        }

        let year = ymd[0]
            .parse()
            .with_context(|| format!("Invalid year: {:?}", ymd[0]))?;
        let month = match ymd.get(1) {
            Some(&t) => Some(
                t.parse()
                    .with_context(|| format!("Invalid month: {:?}", ymd[1]))?,
            ),
            None => None,
        };
        let date = match ymd.get(2) {
            Some(&t) => Some(
                t.parse()
                    .with_context(|| format!("Invalid date: {:?}", ymd[2]))?,
            ),
            None => None,
        };
        Ok(Date { year, month, date })
    }
}
