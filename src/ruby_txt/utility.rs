use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MidashiLevel {
    Oh,   // 大見出し
    Naka, // 中見出し
    Ko,   // 小見出し
}
impl MidashiLevel {
    pub fn of(name: &str) -> Result<Self> {
        match name {
            "大" => Ok(Self::Oh),
            "中" => Ok(Self::Naka),
            "小" => Ok(Self::Ko),
            name => bail!("Unknown midashi level: {}", name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MidashiStyle {
    Normal, // ［＃中見出し］ 等
    Dogyo,  // ［＃同行中見出し］ 等
    Mado,   // ［＃窓中見出し］ 等
}
impl MidashiStyle {
    pub fn of(name: &str) -> Result<Self> {
        match name {
            "" => Ok(Self::Normal),
            "同行" => Ok(Self::Dogyo),
            "窓" => Ok(Self::Mado),
            name => bail!("Unknown midashi style: {}", name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BouDecorationSide {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BouDecorationStyle {
    // 傍点 https://www.aozora.gr.jp/annotation/emphasis.html#boten_chuki
    SesameDotBouten,
    WhiteSesameDotBouten,
    BlackCircleBouten,
    WhiteCircleBouten,
    BlackUpPointingTriangleBouten,
    WhiteUpPointingTriangleBouten,
    BullseyeBouten,
    FisheyeBouten,
    SaltireBouten,

    // 傍線 https://www.aozora.gr.jp/annotation/emphasis.html#bosen_chuki
    SolidBousen,
    DoubleBousen,
    DottedBousen,
    DashedBousen,
    WaveBousen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StringDecorationStyle {
    Bold,
    Italic,
}
