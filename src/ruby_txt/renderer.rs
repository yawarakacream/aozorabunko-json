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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RenderedRubyTxtLine {
    pub components: Vec<RenderedRubyTxtComponent>,
}

impl RenderedRubyTxtLine {
    fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    pub fn push(&mut self, component: RenderedRubyTxtComponent) {
        if let RenderedRubyTxtComponent::String { value } = component {
            self.push_str(&value);
        } else {
            self.components.push(component);
        }
    }

    pub fn push_char(&mut self, ch: char) {
        if let Some(RenderedRubyTxtComponent::String { value }) = self.components.last_mut() {
            value.push(ch);
        } else {
            self.components.push(RenderedRubyTxtComponent::String {
                value: ch.to_string(),
            });
        }
    }

    pub fn push_str(&mut self, string: &str) {
        if let Some(RenderedRubyTxtComponent::String { value }) = self.components.last_mut() {
            value.push_str(string)
        } else {
            self.components.push(RenderedRubyTxtComponent::String {
                value: string.to_string(),
            });
        }
    }

    pub fn pop(&mut self) -> Option<RenderedRubyTxtComponent> {
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

    while !elements.is_empty() {
        match &elements[0] {
            ParsedRubyTxtElement::String { value } => {
                lines.last_mut().unwrap().push_str(&value);
                elements = &elements[1..];
            }

            ParsedRubyTxtElement::NewLine => {
                lines.push(RenderedRubyTxtLine::new());
                elements = &elements[1..];
            }

            ParsedRubyTxtElement::UnknownAnnotation { args } => {
                let args = render_line(&args.iter().map(|a| a).collect::<Vec<_>>())
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
                            let ruby = render_line(&value.iter().map(|v| v).collect::<Vec<_>>())
                                .with_context(|| format!("Failed to render ruby: {:?}", value))?;
                            let children = render_line(&target).with_context(|| {
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
                let ruby = render_line(&value.iter().map(|v| v).collect::<Vec<_>>())
                    .with_context(|| format!("Failed to render ruby: {:?}", value))?;

                let line = lines.last_mut().unwrap();
                let last = line.pop().context("Cannod find String to set ruby")?;
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

fn render_line(elements: &[&ParsedRubyTxtElement]) -> Result<Vec<RenderedRubyTxtComponent>> {
    let lines = render_block(elements)?;
    ensure!(!lines.is_empty(), "Empty block");
    ensure!(lines.len() == 1, "Not line");
    Ok(lines.into_iter().nth(0).unwrap().components)
}
