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
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case", tag = "type")]
    pub enum MidashiLevel {
        Oh,   // 大見出し
        Naka, // 中見出し
        Ko,   // 小見出し
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case", tag = "type")]
    pub enum MidashiStyle {
        Normal, // ［＃中見出し］ 等
        Dogyo,  // ［＃同行中見出し］ 等
        Mado,   // ［＃窓中見出し］ 等
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum BookContentElement {
    String {
        value: String,
    },
    NewLine,

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
}

pub struct BookContentElementList {
    items: Vec<BookContentElementListItem>,
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

    pub fn last_id(&self) -> Option<BookContentElementListItemId> {
        self.items.last().and_then(|item| Some(item.id))
    }

    pub fn pop(&mut self) -> Option<BookContentElement> {
        self.items.pop().and_then(|el| Some(el.element))
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn push(&mut self, element: BookContentElement) {
        self.apply_string_buffer();

        self.items.push(BookContentElementListItem {
            element,
            id: BookContentElementListItemId(self.next_item_id),
        });

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

    pub fn position_by_id(&self, id: &BookContentElementListItemId) -> Option<usize> {
        self.items.iter().position(|item| id == &item.id)
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

    pub fn remove_by_id(&mut self, id: &BookContentElementListItemId) {
        self.items.retain(|item| id != &item.id);
    }

    pub fn collect_to_vec(mut self) -> Vec<BookContentElement> {
        self.apply_string_buffer();

        // String を纏める
        let mut items = Vec::new();
        for item in self.items {
            if let BookContentElement::String { value } = &item.element {
                if let Some(BookContentElement::String { value: last_value }) = items.last_mut() {
                    last_value.push_str(&value);
                    continue;
                }
            }

            items.push(item.element);
        }

        items
    }
}

impl<Idx> std::ops::Index<Idx> for BookContentElementList
where
    Idx: std::slice::SliceIndex<[BookContentElementListItem], Output = BookContentElementListItem>,
{
    type Output = BookContentElementListItem;

    #[inline(always)]
    fn index(&self, index: Idx) -> &Self::Output {
        self.items.index(index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookContentElementListItemId(usize);

#[derive(Debug)]
pub struct BookContentElementListItem {
    id: BookContentElementListItemId,
    element: BookContentElement,
}
