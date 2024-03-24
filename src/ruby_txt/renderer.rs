use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    ruby_txt::parser::{ParsedRubyTxt, ParsedRubyTxtElement},
    utility::CharType,
};

use super::tokenizer::RubyTxtToken;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedRubyTxt {
    pub header: Vec<RenderedRubyTxtLine>,
    pub body: Vec<RenderedRubyTxtLine>,
    pub footer: Vec<RenderedRubyTxtLine>,
}

// 注記などを基に、描画するに適切な構造を求める
pub fn render_ruby_txt(parsed: &ParsedRubyTxt) -> Result<RenderedRubyTxt> {
    let header = render_block(&parsed.header.iter().map(|e| e).collect::<Vec<_>>())?;
    let body = render_block(&parsed.body.iter().map(|e| e).collect::<Vec<_>>())?;
    let footer = render_block(&parsed.footer.iter().map(|e| e).collect::<Vec<_>>())?;
    Ok(RenderedRubyTxt {
        header,
        body,
        footer,
    })
}

// ページに対する状態
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PageStyle {
    Continuous,
    Kaicho,      // 改丁
    Kaipage,     // 改ページ
    Kaimihiraki, // 改見開き
    Kaidan,      // 改段
}

// 字下げ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Jisage {
    level0: usize, // 最初の行
    level1: usize, // 残りの行
}

// 地寄せ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Jiyose {
    level: usize, // 0 なら地付き
    lines: Vec<Vec<RenderedRubyTxtComponent>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RenderedRubyTxtLine {
    page_style: PageStyle,
    jisage: Jisage,

    // 主要素
    components: Vec<RenderedRubyTxtComponent>,

    // 字寄せ
    jiyose: Option<Jiyose>,
}

impl RenderedRubyTxtLine {
    fn new() -> Self {
        Self {
            page_style: PageStyle::Continuous,
            jisage: Jisage {
                level0: 0,
                level1: 0,
            },

            components: Vec::new(),

            jiyose: None,
        }
    }

    fn extract_components(self) -> Result<Vec<RenderedRubyTxtComponent>> {
        ensure!(
            self.page_style == PageStyle::Continuous,
            "page-style is not default"
        );
        ensure!(
            self.jisage
                == Jisage {
                    level0: 0,
                    level1: 0,
                },
            "jisage is not default"
        );
        ensure!(self.jiyose.is_none(), "jiyose is not empty");
        Ok(self.components)
    }

    fn set_page_style(&mut self, page_style: PageStyle) -> Result<()> {
        ensure!(self.is_empty(), "Cannot set pasing to non-empty line");
        ensure!(
            self.page_style == PageStyle::Continuous,
            "page-style already set: {:?}, given {:?}",
            self.page_style,
            page_style
        );
        self.page_style = page_style;
        Ok(())
    }

    fn set_jisage(&mut self, jisage: Jisage) -> Result<()> {
        ensure!(self.is_empty(), "Cannot set jisage to non-empty line");
        ensure!(
            self.jisage
                == Jisage {
                    level0: 0,
                    level1: 0
                },
            "jisage already set: {:?}, given {:?}",
            self.jisage,
            jisage
        );
        self.jisage = jisage;
        Ok(())
    }

    fn set_jiyose(&mut self, jiyose: Jiyose) -> Result<()> {
        ensure!(
            self.jiyose.is_none(),
            "jiyose already set: {:?}, given {:?}",
            self.jiyose,
            jiyose
        );
        self.jiyose = Some(jiyose);
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    fn push(&mut self, component: RenderedRubyTxtComponent) {
        if let RenderedRubyTxtComponent::String { value } = component {
            self.push_str(&value);
        } else {
            self.components.push(component);
        }
    }

    fn push_str(&mut self, string: &str) {
        if let Some(RenderedRubyTxtComponent::String { value }) = self.components.last_mut() {
            value.push_str(string)
        } else {
            self.components.push(RenderedRubyTxtComponent::String {
                value: string.to_string(),
            });
        }
    }

    fn pop(&mut self) -> Option<RenderedRubyTxtComponent> {
        self.components.pop()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum RenderedRubyTxtComponent {
    String {
        value: String,
    },
    UnknownAnnotation {
        // 非空
        args: Vec<RenderedRubyTxtComponent>,
    },

    Ruby {
        ruby: Vec<RenderedRubyTxtComponent>,
        children: Vec<RenderedRubyTxtComponent>,
    },

    Tmp {
        data: ParsedRubyTxtElement,
    },
}

// 注記などを基に、描画するに適切な構造を求める
pub fn render_block(elements: &[&ParsedRubyTxtElement]) -> Result<Vec<RenderedRubyTxtLine>> {
    let mut elements = elements;

    let mut lines = vec![RenderedRubyTxtLine::new()];

    // ブロックで宣言されたレイアウト
    let mut global_jisage: Option<Jisage> = None;

    while !elements.is_empty() {
        match &elements[0] {
            ParsedRubyTxtElement::String { value } => {
                lines.last_mut().unwrap().push_str(&value);
                elements = &elements[1..];
            }

            ParsedRubyTxtElement::NewLine => {
                let mut line = RenderedRubyTxtLine::new();

                if let Some(global_jisage) = &global_jisage {
                    line.set_jisage(global_jisage.clone()).unwrap();
                }
                lines.push(line);

                elements = &elements[1..];
            }

            ParsedRubyTxtElement::UnknownAnnotation { args } => {
                let args = render_line_components(&args.iter().map(|a| a).collect::<Vec<_>>())
                    .with_context(|| format!("Failed to render unknown annotation: {:?}", args))?;

                lines
                    .last_mut()
                    .unwrap()
                    .push(RenderedRubyTxtComponent::UnknownAnnotation { args });
                elements = &elements[1..];
            }

            ParsedRubyTxtElement::PositionMarker => {
                elements = &elements[1..];

                let line = lines.last_mut().unwrap();
                let mut target = Vec::new();

                let mut elements_for_marker = elements;
                let is_marker = loop {
                    if elements_for_marker.is_empty() {
                        break false;
                    }

                    match elements_for_marker[0] {
                        ParsedRubyTxtElement::NewLine => break false,

                        ParsedRubyTxtElement::Ruby { value } => {
                            let ruby = render_line_components(
                                &value.iter().map(|v| v).collect::<Vec<_>>(),
                            )
                            .with_context(|| format!("Failed to render ruby: {:?}", value))?;
                            let children = render_line_components(&target).with_context(|| {
                                format!("Failed to render ruby children: {:?}", value)
                            })?;
                            line.push(RenderedRubyTxtComponent::Ruby { ruby, children });
                            elements_for_marker = &elements_for_marker[1..];
                            break true;
                        }

                        _ => {
                            target.push(elements_for_marker[0]);
                            elements_for_marker = &elements_for_marker[1..];
                        }
                    }
                };

                if is_marker {
                    elements = elements_for_marker;
                } else {
                    // PositionMarker でないなら文字列に戻す
                    line.push_str(RubyTxtToken::PositionMarker.to_str());
                }
            }

            ParsedRubyTxtElement::Ruby { value } => {
                let ruby = render_line_components(&value.iter().map(|v| v).collect::<Vec<_>>())
                    .with_context(|| format!("Failed to render ruby: {:?}", value))?;

                let line = lines.last_mut().unwrap();
                let last = line
                    .pop()
                    .with_context(|| format!("Cannod find elements to set ruby {:?}", ruby))?;
                match last {
                    RenderedRubyTxtComponent::String { value } => {
                        let value_chars: Vec<_> = value.chars().collect();

                        let mut ruby_start_index = value_chars.len();
                        let last_char_type = CharType::from(*value_chars.last().unwrap());
                        for c in value_chars.iter().rev() {
                            if CharType::from(*c) != last_char_type {
                                break;
                            }
                            ruby_start_index -= 1;
                        }

                        if 0 < ruby_start_index {
                            line.push(RenderedRubyTxtComponent::String {
                                value: value_chars[..ruby_start_index].iter().collect(),
                            });
                        }
                        line.push(RenderedRubyTxtComponent::Ruby {
                            ruby,
                            children: vec![RenderedRubyTxtComponent::String {
                                value: value_chars[ruby_start_index..].iter().collect(),
                            }],
                        });
                    }

                    // 不明な外字注記にルビが振られることがある
                    RenderedRubyTxtComponent::UnknownAnnotation { args: _ } => {
                        line.push(RenderedRubyTxtComponent::Ruby {
                            ruby,
                            children: vec![last],
                        });
                    }

                    // TODO: 画像にルビが振られることがある
                    _ => bail!("Cannot set ruby to {:?}", last),
                };

                elements = &elements[1..];
            }

            ParsedRubyTxtElement::KaichoAttention => {
                elements = &elements[1..];
                if elements.is_empty() {
                    continue;
                }

                ensure!(
                    matches!(elements[0], ParsedRubyTxtElement::NewLine),
                    "Invalid kaicho"
                );
                elements = &elements[1..];

                lines
                    .last_mut()
                    .unwrap()
                    .set_page_style(PageStyle::Kaicho)?;
            }

            ParsedRubyTxtElement::KaipageAttention => {
                elements = &elements[1..];
                if elements.is_empty() {
                    continue;
                }

                ensure!(
                    matches!(elements[0], ParsedRubyTxtElement::NewLine),
                    "Invalid kaipage"
                );
                elements = &elements[1..];

                lines
                    .last_mut()
                    .unwrap()
                    .set_page_style(PageStyle::Kaipage)?;
            }

            ParsedRubyTxtElement::KaimihirakiAttention => {
                elements = &elements[1..];
                if elements.is_empty() {
                    continue;
                }

                ensure!(
                    matches!(elements[0], ParsedRubyTxtElement::NewLine),
                    "Invalid kaimihiraki"
                );
                elements = &elements[1..];

                lines
                    .last_mut()
                    .unwrap()
                    .set_page_style(PageStyle::Kaimihiraki)?;
            }

            ParsedRubyTxtElement::KaidanAttention => {
                elements = &elements[1..];
                if elements.is_empty() {
                    continue;
                }

                ensure!(
                    matches!(elements[0], ParsedRubyTxtElement::NewLine),
                    "Invalid kaidan"
                );
                elements = &elements[1..];

                lines
                    .last_mut()
                    .unwrap()
                    .set_page_style(PageStyle::Kaidan)?;
            }

            ParsedRubyTxtElement::JisageAnnotation { level } => {
                elements = &elements[1..];

                ensure!(lines.pop().unwrap().is_empty(), "Invalid one-line jisage");

                let jisage = if let Some(global_jisage) = &global_jisage {
                    Jisage {
                        level0: *level + global_jisage.level0,
                        level1: *level + global_jisage.level1,
                    }
                } else {
                    Jisage {
                        level0: *level,
                        level1: *level,
                    }
                };

                let mut line = RenderedRubyTxtLine::new();
                line.set_jisage(jisage)?;
                lines.push(line);
            }

            ParsedRubyTxtElement::JisageStartAnnotation { level } => {
                ensure!(lines.pop().unwrap().is_empty(), "Invalid jisage-start");
                elements = &elements[1..];

                global_jisage = Some(Jisage {
                    level0: *level,
                    level1: *level,
                });
            }

            ParsedRubyTxtElement::JisageWithOrikaeshiStartAnnotation { level0, level1 } => {
                ensure!(
                    lines.pop().unwrap().is_empty(),
                    "Invalid jisage-with-orikaeshi-start"
                );
                elements = &elements[1..];

                global_jisage = Some(Jisage {
                    level0: *level0,
                    level1: *level1,
                });
            }

            ParsedRubyTxtElement::JisageAfterTentsukiStartAnnotation { level } => {
                ensure!(
                    lines.pop().unwrap().is_empty(),
                    "Invalid jisage-after-tentsuki-start"
                );
                elements = &elements[1..];

                global_jisage = Some(Jisage {
                    level0: 0,
                    level1: *level,
                });
            }

            ParsedRubyTxtElement::JisageEndAnnotation => {
                ensure!(lines.pop().unwrap().is_empty(), "Invalid jisage-end");

                // 規格外の注記で字下げが始まっている可能性があるのでエラーにしない
                elements = &elements[1..];
                global_jisage = None;
            }

            ParsedRubyTxtElement::JitsukiAnnotation => {
                elements = &elements[1..];

                let mut jitsuki_elements = Vec::new();
                while !elements.is_empty() {
                    if matches!(elements[0], ParsedRubyTxtElement::NewLine) {
                        break;
                    }
                    jitsuki_elements.push(elements[0]);
                    elements = &elements[1..];
                }

                let jitsuki_line = render_line_components(&jitsuki_elements)
                    .context("Failed to render a line with jitsuki")?;
                lines.last_mut().unwrap().set_jiyose(Jiyose {
                    level: 0,
                    lines: vec![jitsuki_line],
                })?;
            }

            ParsedRubyTxtElement::JitsukiStartAnnotation => {
                ensure!(lines.pop().unwrap().is_empty(), "Invalid jitsuki-start");
                ensure!(
                    matches!(elements.get(1), Some(ParsedRubyTxtElement::NewLine)),
                    "Invalid jitsuki-start"
                );
                elements = &elements[2..];

                let mut jitsuki_elements = Vec::new();
                while !elements.is_empty() {
                    let el = elements[0];
                    elements = &elements[1..];

                    if matches!(el, ParsedRubyTxtElement::JitsukiEndAnnotation) {
                        break;
                    }
                    jitsuki_elements.push(el);
                }

                // "［＃ここで地付き終わり］" 前の改行を取り除く
                ensure!(
                    matches!(
                        jitsuki_elements.pop().context("Empty jitsuki block")?,
                        ParsedRubyTxtElement::NewLine
                    ),
                    "Invalid jitsuki-end"
                );

                // 地付きブロックは全行を既にある 1 行に入れる
                let jitsuki_lines: Result<Vec<_>> = render_block(&jitsuki_elements)?
                    .into_iter()
                    .map(|line| line.extract_components())
                    .collect();
                lines.last_mut().unwrap().set_jiyose(Jiyose {
                    level: 0,
                    lines: jitsuki_lines?,
                })?;
            }

            ParsedRubyTxtElement::JitsukiEndAnnotation => {
                // 規格外の注記で地付きが始まっている可能性があるのでエラーにしない
                elements = &elements[1..];
            }

            ParsedRubyTxtElement::JiyoseAnnotation { level } => {
                elements = &elements[1..];

                let mut jiyose_elements = Vec::new();
                while !elements.is_empty() {
                    if matches!(elements[0], ParsedRubyTxtElement::NewLine) {
                        break;
                    }
                    jiyose_elements.push(elements[0]);
                    elements = &elements[1..];
                }

                let jiyose_line = render_line_components(&jiyose_elements)
                    .context("Failed to render a line with jiyose")?;
                lines.last_mut().unwrap().set_jiyose(Jiyose {
                    level: *level,
                    lines: vec![jiyose_line],
                })?;
            }

            ParsedRubyTxtElement::JiyoseStartAnnotation { level } => {
                ensure!(lines.pop().unwrap().is_empty(), "Invalid jiyose-start");
                ensure!(
                    matches!(elements.get(1), Some(ParsedRubyTxtElement::NewLine)),
                    "Invalid jiyose-start"
                );
                elements = &elements[2..];

                let mut jiyose_elements = Vec::new();
                while !elements.is_empty() {
                    let el = elements[0];
                    elements = &elements[1..];

                    if matches!(el, ParsedRubyTxtElement::JiyoseEndAnnotation) {
                        break;
                    }
                    jiyose_elements.push(el);
                }

                // "［＃ここで字上げ終わり］" 前の改行を取り除く
                ensure!(
                    matches!(
                        jiyose_elements.pop().context("Empty jiyose block")?,
                        ParsedRubyTxtElement::NewLine
                    ),
                    "Invalid jiyose-end"
                );

                // 地寄せブロックは 1 行につき 1 行
                for jiyose_line in render_block(&jiyose_elements)? {
                    let jiyose_line = jiyose_line.extract_components()?;

                    let mut line = RenderedRubyTxtLine::new();
                    line.set_jiyose(Jiyose {
                        level: *level,
                        lines: vec![jiyose_line],
                    })?;
                    lines.push(line);
                }
            }

            ParsedRubyTxtElement::JiyoseEndAnnotation => {
                // 規格外の注記で地寄せが始まっている可能性があるのでエラーにしない
                elements = &elements[1..];
            }

            _ => {
                lines
                    .last_mut()
                    .unwrap()
                    .push(RenderedRubyTxtComponent::Tmp {
                        data: elements[0].clone(),
                    });
                elements = &elements[1..];
            }
        }
    }

    while let Some(last) = lines.last() {
        if !last.is_empty() {
            break;
        }
        lines.pop();
    }

    Ok(lines)
}

fn render_line_components(
    elements: &[&ParsedRubyTxtElement],
) -> Result<Vec<RenderedRubyTxtComponent>> {
    let lines = render_block(elements)?;
    ensure!(!lines.is_empty(), "Empty block");
    ensure!(lines.len() == 1, "Not line");

    lines.into_iter().nth(0).unwrap().extract_components()
}
