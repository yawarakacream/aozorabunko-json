use anyhow::{bail, ensure, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    ruby_txt::{
        block_parser::parse_block,
        parser::ParsedRubyTxtElement,
        tokenizer::RubyTxtToken,
        utility::{
            BouDecorationSide, BouDecorationStyle, MidashiLevel, MidashiStyle,
            StringDecorationStyle,
        },
    },
    utility::str::parse_number,
};

// AnnotationStart ... AnnotationEnd
pub(super) fn parse_annotation<'a>(
    tokens: &'a [&'a RubyTxtToken],
) -> Result<(&'a [&'a RubyTxtToken], Option<ParsedRubyTxtElement>)> {
    ensure!(matches!(tokens.get(0), Some(RubyTxtToken::AnnotationStart)));
    let tokens = &tokens[1..];

    let end_index = {
        let mut end_index = None;
        let mut level = 0;
        for (i, &token) in tokens.iter().enumerate() {
            match token {
                &RubyTxtToken::AnnotationStart | &RubyTxtToken::GaijiAnnotationStart => {
                    level += 1;
                }
                &RubyTxtToken::AnnotationEnd => {
                    if level == 0 {
                        end_index = Some(i);
                        break;
                    }
                    level -= 1;
                }
                &RubyTxtToken::NewLine => break,
                _ => continue,
            }
        }
        end_index
    }
    .context("A line ends without '］'")?;

    let args = &tokens[..end_index];
    let tokens = &tokens[(end_index + 1)..];

    let args = parse_block(args)?;

    // もっとうまい分岐の仕方がある？
    let annotation = (|| {
        // 空の annotation は "［＃］：入力者注　主に外字の説明や、傍点の位置の指定" のように使われることがある
        if args.len() == 0 {
            return Ok(Some(ParsedRubyTxtElement::String {
                value: "［＃］".to_owned(),
            }));
        }

        let first_arg = match args.first().unwrap() {
            ParsedRubyTxtElement::String { value } => value,
            _ => bail!("Unknown annotation: {:?}", args),
        };

        let last_arg = match args.last().unwrap() {
            ParsedRubyTxtElement::String { value } => value,
            _ => bail!("Unknown annotation: {:?}", args),
        };

        if first_arg.starts_with("「") {
            // ［＃「○○」に「ママ」の注記］
            if last_arg.ends_with("」に「ママ」の注記") {
                return Ok(None);
            }

            // ［＃「○○」は底本では「●●」］
            for arg in &args {
                if let ParsedRubyTxtElement::String { value } = arg {
                    if value.contains("」は底本では「") && last_arg.ends_with("」") {
                        return Ok(None);
                    }
                }
            }

            // ［＃「○○」はママ］
            // ［＃ルビの「○○」はママ］
            if last_arg.ends_with("」はママ") {
                return Ok(None);
            }
        }

        // // 底本に関する注記は例外がかなり多いので `底本では` を含むものをすべて無視する
        // // TODO: よくないと思うのでなんとかする
        // for arg in &args {
        //     if let BookContentElement::String { value } = arg {
        //         if value.contains("底本では") {
        //             return Ok(None);
        //         }
        //     }
        // }

        // "「Vec<BookContentElement>」String" 型
        if first_arg.starts_with('「') && last_arg.contains('」') {
            let target = match args.len() {
                1 => {
                    let l = "「".len();
                    let r = first_arg.rfind('」').unwrap();
                    vec![ParsedRubyTxtElement::String {
                        value: first_arg[l..r].to_string(),
                    }]
                }

                _ => {
                    ensure!(args.len() != 2, "Invalid bou decoration: {:?}", args);

                    let first = if "「".len() < first_arg.len() {
                        Some(ParsedRubyTxtElement::String {
                            value: first_arg["「".len()..].to_string(),
                        })
                    } else {
                        None
                    };

                    let last = {
                        let r = last_arg.rfind('」').unwrap();
                        if 0 < r {
                            Some(ParsedRubyTxtElement::String {
                                value: last_arg[..r].to_string(),
                            })
                        } else {
                            None
                        }
                    };

                    let mut target = Vec::new();

                    if let Some(first) = first {
                        target.push(first);
                    }

                    for arg in &args[1..(args.len() - 1)] {
                        target.push(arg.clone());
                    }

                    if let Some(last) = last {
                        target.push(last);
                    }

                    target
                }
            };

            let annotation_name = last_arg[last_arg.rfind('」').unwrap()..].to_string();

            static REGEX_BOU_DECORATION: Lazy<Regex> =
                Lazy::new(|| Regex::new(r"」(?P<left>の左)?に(?P<style>.*(点|線))$").unwrap());
            if let Some(caps) = REGEX_BOU_DECORATION.captures(&annotation_name) {
                let side = match caps.name("left") {
                    Some(left) => {
                        assert_eq!(left.as_str(), "の左");
                        BouDecorationSide::Left
                    }
                    None => BouDecorationSide::Right,
                };
                let style = match bou_decoration_style_of(caps.name("style").unwrap().as_str()) {
                    Ok(style) => style,
                    Err(_) => return Ok(Some(ParsedRubyTxtElement::UnknownAnnotation { args })),
                };

                return Ok(Some(ParsedRubyTxtElement::BouDecoration {
                    target,
                    style,
                    side,
                }));
            }

            if annotation_name == "は太字" {
                return Ok(Some(ParsedRubyTxtElement::StringDecoration {
                    target,
                    style: StringDecorationStyle::Bold,
                }));
            }

            if annotation_name == "は斜体" {
                return Ok(Some(ParsedRubyTxtElement::StringDecoration {
                    target,
                    style: StringDecorationStyle::Italic,
                }));
            }

            if annotation_name == "はキャプション" {
                return Ok(Some(ParsedRubyTxtElement::Caption { value: target }));
            }
        }

        // TODO
        if 1 < args.len() {
            return Ok(Some(ParsedRubyTxtElement::UnknownAnnotation { args }));
        }

        // 1 文字列のもの
        ensure!(args.len() == 1, "Unknown annotation: {:?}", args);
        let arg = match &args[0] {
            ParsedRubyTxtElement::String { value } => value,
            arg => bail!("Unknown annotation: {:?}", arg),
        };

        if arg == "改丁" {
            return Ok(Some(ParsedRubyTxtElement::KaichoAttention));
        }

        if arg == "改ページ" {
            return Ok(Some(ParsedRubyTxtElement::KaipageAttention));
        }

        if arg == "改見開き" {
            return Ok(Some(ParsedRubyTxtElement::KaimihirakiAttention));
        }

        if arg == "改段" {
            return Ok(Some(ParsedRubyTxtElement::KaidanAttention));
        }

        static REGEX_JISAGE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^(?P<level>[０-９]+)字下げ$").unwrap());
        if let Some(caps) = REGEX_JISAGE.captures(&arg) {
            let level = parse_number(caps.name("level").unwrap().as_str())
                .with_context(|| format!("Failed to parse {:?}", arg))?;
            return Ok(Some(ParsedRubyTxtElement::JisageAnnotation { level }));
        }

        static REGEX_JISAGE_START: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^ここから(?P<level>[０-９]+)字下げ$").unwrap());
        if let Some(caps) = REGEX_JISAGE_START.captures(&arg) {
            let level = parse_number(caps.name("level").unwrap().as_str())
                .with_context(|| format!("Failed to parse {:?}", arg))?;
            return Ok(Some(ParsedRubyTxtElement::JisageStartAnnotation { level }));
        }

        static REGEX_JISAGE_WITH_ORIKAESHI_START: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"^ここから(?P<level0>[０-９]+)字下げ、折り返して(?P<level1>[０-９]+)字下げ$",
            )
            .unwrap()
        });
        if let Some(caps) = REGEX_JISAGE_WITH_ORIKAESHI_START.captures(&arg) {
            let level0 = parse_number(caps.name("level0").unwrap().as_str())
                .with_context(|| format!("Failed to parse {:?}", arg))?;
            let level1 = parse_number(caps.name("level1").unwrap().as_str())
                .with_context(|| format!("Failed to parse {:?}", arg))?;
            return Ok(Some(
                ParsedRubyTxtElement::JisageWithOrikaeshiStartAnnotation { level0, level1 },
            ));
        }

        static REGEX_JISAGE_AFTER_TENTSUKI_START: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^ここから改行天付き、折り返して(?P<level>[０-９]+)字下げ$").unwrap()
        });
        if let Some(caps) = REGEX_JISAGE_AFTER_TENTSUKI_START.captures(&arg) {
            let level = parse_number(caps.name("level").unwrap().as_str())
                .with_context(|| format!("Failed to parse {:?}", arg))?;
            return Ok(Some(
                ParsedRubyTxtElement::JisageAfterTentsukiStartAnnotation { level },
            ));
        }

        if arg == "ここで字下げ終わり" {
            return Ok(Some(ParsedRubyTxtElement::JisageEndAnnotation));
        }

        if arg == "地付き" {
            return Ok(Some(ParsedRubyTxtElement::JitsukiAnnotation));
        }

        if arg == "ここから地付き" {
            return Ok(Some(ParsedRubyTxtElement::JitsukiStartAnnotation));
        }

        if arg == "ここで地付き終わり" {
            return Ok(Some(ParsedRubyTxtElement::JitsukiEndAnnotation));
        }

        static REGEX_JIYOSE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^地から(?P<level>[０-９]+)字上げ$").unwrap());
        if let Some(caps) = REGEX_JIYOSE.captures(&arg) {
            let level = parse_number(caps.name("level").unwrap().as_str())
                .with_context(|| format!("Failed to parse {:?}", arg))?;
            return Ok(Some(ParsedRubyTxtElement::JiyoseAnnotation { level }));
        }

        static REGEX_JIYOSE_START: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^ここから地から(?P<level>[０-９]+)字上げ$").unwrap());
        if let Some(caps) = REGEX_JIYOSE_START.captures(&arg) {
            let level = parse_number(caps.name("level").unwrap().as_str())
                .with_context(|| format!("Failed to parse {:?}", arg))?;
            return Ok(Some(ParsedRubyTxtElement::JiyoseStartAnnotation { level }));
        }

        if arg == "ここで字上げ終わり" {
            return Ok(Some(ParsedRubyTxtElement::JiyoseEndAnnotation));
        }

        if arg == "ページの左右中央" {
            return Ok(Some(ParsedRubyTxtElement::PageCenterAnnotation));
        }

        static REGEX_MIDASHI: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^「(?P<value>.+)」は(?P<style>同行|窓)?(?P<level>大|中|小)見出し$")
                .unwrap()
        });
        if let Some(caps) = REGEX_MIDASHI.captures(&arg) {
            let value = caps.name("value").unwrap().as_str().to_owned();
            let style = MidashiStyle::of(caps.name("style").map_or("", |m| m.as_str()))?;
            let level = MidashiLevel::of(caps.name("level").unwrap().as_str())?;
            return Ok(Some(ParsedRubyTxtElement::Midashi {
                value,
                style,
                level,
            }));
        }

        static REGEX_MIDASHI_START: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^(ここから)?(?P<style>同行|窓)?(?P<level>大|中|小)見出し$").unwrap()
        });
        if let Some(caps) = REGEX_MIDASHI_START.captures(&arg) {
            let style = MidashiStyle::of(caps.name("style").map_or("", |m| m.as_str()))?;
            let level = MidashiLevel::of(caps.name("level").unwrap().as_str())?;
            return Ok(Some(ParsedRubyTxtElement::MidashiStart { level, style }));
        }

        static REGEX_MIDASHI_END: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^.*見出し終わり$").unwrap());
        if REGEX_MIDASHI_END.is_match(&arg) {
            return Ok(Some(ParsedRubyTxtElement::MidashiEnd));
        }

        static REGEX_KAERITEN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^(?P<ichini>一|二|三|四)?(?P<jouge>上|中|下)?(?P<kouotsu>甲|乙|丙|丁)?(?P<re>レ)?$").unwrap()
        });
        if let Some(caps) = REGEX_KAERITEN.captures(&arg) {
            let ichini = match caps.name("ichini") {
                Some(ichini) => match ichini.as_str() {
                    "一" => Some(0),
                    "二" => Some(1),
                    "三" => Some(2),
                    "四" => Some(3),
                    _ => panic!(),
                },
                None => None,
            };
            let jouge = match caps.name("jouge") {
                Some(jouge) => match jouge.as_str() {
                    "上" => Some(0),
                    "中" => Some(1),
                    "下" => Some(2),
                    _ => panic!(),
                },
                None => None,
            };
            let kouotsu = match caps.name("kouotsu") {
                Some(kouotsu) => match kouotsu.as_str() {
                    "甲" => Some(0),
                    "乙" => Some(1),
                    "丙" => Some(2),
                    "丁" => Some(3),
                    _ => panic!(),
                },
                None => None,
            };
            let re = match caps.name("re") {
                Some(re) => match re.as_str() {
                    "レ" => true,
                    _ => panic!(),
                },
                None => false,
            };
            return Ok(Some(ParsedRubyTxtElement::Kaeriten {
                ichini,
                jouge,
                kouotsu,
                re,
            }));
        }

        static REGEX_KUNTEN_OKURIGANA: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^（(?P<kana>.+)）$").unwrap());
        if let Some(caps) = REGEX_KUNTEN_OKURIGANA.captures(&arg) {
            let kana = caps.name("kana").unwrap().as_str();
            return Ok(Some(ParsedRubyTxtElement::KuntenOkurigana {
                value: kana.to_owned(),
            }));
        }

        static REGEX_BOU_DECORATION_START: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^(?P<left>左に)?(?P<style>.*(点|線))$").unwrap());
        if let Some(caps) = REGEX_BOU_DECORATION_START.captures(&arg) {
            let side = match caps.name("left") {
                Some(left) => {
                    assert_eq!(left.as_str(), "左に");
                    BouDecorationSide::Left
                }
                None => BouDecorationSide::Right,
            };
            let style = match bou_decoration_style_of(caps.name("style").unwrap().as_str()) {
                Ok(style) => style,
                Err(_) => return Ok(Some(ParsedRubyTxtElement::UnknownAnnotation { args })),
            };
            return Ok(Some(ParsedRubyTxtElement::BouDecorationStart {
                style,
                side,
            }));
        }

        static REGEX_BOU_DECORATION_END: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^(?P<left>左に)?(?P<style>.*(点|線))終わり$").unwrap());
        if let Some(caps) = REGEX_BOU_DECORATION_END.captures(&arg) {
            let side = match caps.name("left") {
                Some(left) => {
                    assert_eq!(left.as_str(), "左に");
                    BouDecorationSide::Left
                }
                None => BouDecorationSide::Right,
            };
            let style = match bou_decoration_style_of(caps.name("style").unwrap().as_str()) {
                Ok(style) => style,
                Err(_) => return Ok(Some(ParsedRubyTxtElement::UnknownAnnotation { args })),
            };
            return Ok(Some(ParsedRubyTxtElement::BouDecorationEnd { style, side }));
        }

        if arg == "太字" || arg == "ここから太字" {
            return Ok(Some(ParsedRubyTxtElement::StringDecorationStart {
                style: StringDecorationStyle::Bold,
            }));
        }

        if arg == "太字終わり" || arg == "ここで太字終わり" {
            return Ok(Some(ParsedRubyTxtElement::StringDecorationEnd {
                style: StringDecorationStyle::Bold,
            }));
        }

        if arg == "斜体" || arg == "ここから斜体" {
            return Ok(Some(ParsedRubyTxtElement::StringDecorationStart {
                style: StringDecorationStyle::Italic,
            }));
        }

        if arg == "斜体終わり" || arg == "ここで斜体終わり" {
            return Ok(Some(ParsedRubyTxtElement::StringDecorationEnd {
                style: StringDecorationStyle::Italic,
            }));
        }

        static REGEX_IMAGE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"^(?P<alt>.+)（(?P<path>fig[0-9]+_[0-9]+\.png)(、横[0-9]+×縦[0-9]+)?）入る$",
            )
            .unwrap()
        });
        if let Some(caps) = REGEX_IMAGE.captures(&arg) {
            let path = caps.name("path").unwrap().as_str().to_owned();
            let alt = caps.name("alt").unwrap().as_str().to_owned();
            return Ok(Some(ParsedRubyTxtElement::Image { path, alt }));
        }

        if arg == "キャプション" {
            return Ok(Some(ParsedRubyTxtElement::CaptionStart));
        }

        if arg == "キャプション終わり" {
            return Ok(Some(ParsedRubyTxtElement::CaptionEnd));
        }

        if arg == "割り注" {
            return Ok(Some(ParsedRubyTxtElement::WarichuStart));
        }

        if arg == "割り注終わり" {
            return Ok(Some(ParsedRubyTxtElement::WarichuEnd));
        }

        Ok(Some(ParsedRubyTxtElement::UnknownAnnotation { args }))
    })()?;

    Ok((tokens, annotation))
}

fn bou_decoration_style_of(name: &str) -> Result<BouDecorationStyle> {
    match name {
        "傍点" => Ok(BouDecorationStyle::SesameDotBouten),
        "白ゴマ傍点" => Ok(BouDecorationStyle::WhiteSesameDotBouten),
        "丸傍点" => Ok(BouDecorationStyle::BlackCircleBouten),
        "白丸傍点" => Ok(BouDecorationStyle::WhiteCircleBouten),
        "黒三角傍点" => Ok(BouDecorationStyle::BlackUpPointingTriangleBouten),
        "白三角傍点" => Ok(BouDecorationStyle::WhiteUpPointingTriangleBouten),
        "二重丸傍点" => Ok(BouDecorationStyle::BullseyeBouten),
        "蛇の目傍点" => Ok(BouDecorationStyle::FisheyeBouten),
        "ばつ傍点" => Ok(BouDecorationStyle::SaltireBouten),
        "傍線" => Ok(BouDecorationStyle::SolidBousen),
        "二重傍線" => Ok(BouDecorationStyle::DoubleBousen),
        "鎖線" => Ok(BouDecorationStyle::DottedBousen),
        "破線" => Ok(BouDecorationStyle::DashedBousen),
        "波線" => Ok(BouDecorationStyle::WaveBousen),
        name => bail!("Unknown bou-decoration style: {}", name),
    }
}
