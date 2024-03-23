use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BookContentOriginalDataType {
    RubyTxt,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookContent {
    pub original_data_type: BookContentOriginalDataType,
    pub header: Vec<BookContentElement>,
    pub body: Vec<BookContentElement>,
    pub footer: Vec<BookContentElement>,
}

pub mod book_content_element_util {
    use anyhow::{bail, Result};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum BouDecorationSide {
        Left,
        Right,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum StringDecorationStyle {
        Bold,
        Italic,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum BookContentElement {
    String {
        value: String,
    },
    NewLine,
    UnknownAnnotation {
        args: Vec<BookContentElement>,
    },

    RubyStart {
        value: String,
    },
    RubyEnd,

    KaichoAttention,      // ［＃改丁］
    KaipageAttention,     // ［＃改ページ］
    KaimihirakiAttention, // ［＃改見開き］
    KaidanAttention,      // ［＃改段］

    // ［＃○字下げ］ => { level: ○ }
    JisageAnnotation {
        level: usize,
    },
    // ［＃ここから○字下げ］ => { level: ○ }
    JisageStartAnnotation {
        level: usize,
    },
    // ［＃ここから○字下げ、折り返して●字下げ］ => { level0: ○, level1: ● }
    JisageWithOrikaeshiStartAnnotation {
        level0: usize,
        level1: usize,
    },
    // ［＃ここから改行天付き、折り返して○字下げ］ => { level: ○ }
    JisageAfterTentsukiStartAnnotation {
        level: usize,
    },
    // ［＃ここで字下げ終わり］
    JisageEndAnnotation,

    // ［＃地付き］
    JitsukiAnnotation,
    // ［＃ここから地付き］
    JitsukiStartAnnotation,
    // ［＃ここで地付き終わり］
    JitsukiEndAnnotation,

    // ［＃地から○字上げ］
    JiyoseAnnotation {
        level: usize,
    },
    // ［＃ここから地から○字上げ］
    JiyoseStartAnnotation {
        level: usize,
    },
    // ［＃ここで字上げ終わり］
    JiyoseEndAnnotation,

    // ［＃ページの左右中央］
    PageCenterAnnotation,

    // 見出し
    Midashi {
        value: String,
        level: book_content_element_util::MidashiLevel,
        style: book_content_element_util::MidashiStyle,
    },
    MidashiStart {
        level: book_content_element_util::MidashiLevel,
        style: book_content_element_util::MidashiStyle,
    },
    MidashiEnd {
        level: book_content_element_util::MidashiLevel,
        style: book_content_element_util::MidashiStyle,
    },

    // 返り点
    Kaeriten {
        // 0:［＃一］, 1:［＃二］, 2:［＃三］, 3:［＃四］
        ichini: Option<usize>,
        // 0:［＃上］, 1:［＃中］, 2:［＃下］
        jouge: Option<usize>,
        // 0:［＃甲］, 1:［＃乙］, 2:［＃丙］, 3:［＃丁］
        kouotsu: Option<usize>,
        // false: なし, true:［＃レ］
        re: bool,
    },
    // ［＃（○○）］
    KuntenOkurigana {
        value: String,
    },

    // 傍点・傍線
    BouDecoration {
        target: Vec<BookContentElement>,
        side: book_content_element_util::BouDecorationSide,
        style: book_content_element_util::BouDecorationStyle,
    },
    BouDecorationStart {
        side: book_content_element_util::BouDecorationSide,
        style: book_content_element_util::BouDecorationStyle,
    },
    BouDecorationEnd {
        side: book_content_element_util::BouDecorationSide,
        style: book_content_element_util::BouDecorationStyle,
    },

    // 太字・斜体
    StringDecoration {
        target: Vec<BookContentElement>,
        style: book_content_element_util::StringDecorationStyle,
    },
    StringDecorationStart {
        style: book_content_element_util::StringDecorationStyle,
    },
    StringDecorationEnd {
        style: book_content_element_util::StringDecorationStyle,
    },

    // ［＃○○（●●.png）入る］
    Image {
        path: String,
        alt: String,
    },
    // ［＃「○○」はキャプション］
    Caption {
        value: Vec<BookContentElement>,
    },
    // ［＃キャプション］
    CaptionStart,
    // ［＃キャプション終わり］
    CaptionEnd,

    // ［＃割り注］
    WarichuStart,
    // ［＃割り注終わり］
    WarichuEnd,
}

pub struct BookContentElementList {
    items: Vec<BookContentElement>,
    string_buffer: String,

    next_item_id: usize,
}

impl BookContentElementList {
    pub fn new() -> Self {
        BookContentElementList {
            items: Vec::new(),
            string_buffer: String::new(),

            next_item_id: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn push(&mut self, element: BookContentElement) {
        self.apply_string_buffer();

        self.items.push(element);

        self.next_item_id += 1;
    }

    pub fn push_char(&mut self, value: char) {
        self.string_buffer.push(value)
    }

    pub fn push_str(&mut self, value: &str) {
        self.string_buffer.push_str(&value);
    }

    pub fn extend(&mut self, elements: Vec<BookContentElement>) {
        for el in elements {
            if let BookContentElement::String { value } = el {
                self.push_str(&value);
            } else {
                self.push(el);
            }
        }
    }

    pub fn pop(&mut self) -> Option<BookContentElement> {
        self.items.pop()
    }

    pub fn apply_string_buffer(&mut self) {
        if self.string_buffer.is_empty() {
            return;
        }

        let string_buffer = self.string_buffer.clone();
        self.string_buffer.clear();

        self.push(BookContentElement::String {
            value: string_buffer,
        });
    }

    pub fn collect_to_vec(mut self) -> Vec<BookContentElement> {
        self.apply_string_buffer();

        // String を纏める
        let mut items = Vec::new();
        for item in self.items {
            if let BookContentElement::String { value } = &item {
                if let Some(BookContentElement::String { value: last_value }) = items.last_mut() {
                    last_value.push_str(&value);
                    continue;
                }
            }

            items.push(item);
        }

        items
    }
}

impl<Idx> std::ops::Index<Idx> for BookContentElementList
where
    Idx: std::slice::SliceIndex<[BookContentElement], Output = BookContentElement>,
{
    type Output = BookContentElement;

    #[inline(always)]
    fn index(&self, index: Idx) -> &Self::Output {
        self.items.index(index)
    }
}
