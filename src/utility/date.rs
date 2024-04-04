use anyhow::{anyhow, Context, Result};
use serde::Serialize;

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
