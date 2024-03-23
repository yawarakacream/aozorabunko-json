use crate::ruby_txt::parser::ParsedRubyTxtElement;

pub struct ParsedRubyTxtElementList {
    items: Vec<ParsedRubyTxtElement>,
}

impl ParsedRubyTxtElementList {
    pub fn new() -> Self {
        ParsedRubyTxtElementList { items: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn push(&mut self, element: ParsedRubyTxtElement) {
        if let ParsedRubyTxtElement::String { value } = element {
            self.push_str(&value);
        } else {
            self.items.push(element);
        }
    }

    pub fn push_char(&mut self, ch: char) {
        if let Some(ParsedRubyTxtElement::String { value }) = self.items.last_mut() {
            value.push(ch);
        } else {
            self.items.push(ParsedRubyTxtElement::String {
                value: ch.to_string(),
            });
        }
    }

    pub fn push_str(&mut self, string: &str) {
        if let Some(ParsedRubyTxtElement::String { value }) = self.items.last_mut() {
            value.push_str(string)
        } else {
            self.items.push(ParsedRubyTxtElement::String {
                value: string.to_string(),
            });
        }
    }

    pub fn extend(&mut self, elements: Vec<ParsedRubyTxtElement>) {
        self.items.extend(elements);
    }

    pub fn pop(&mut self) -> Option<ParsedRubyTxtElement> {
        self.items.pop()
    }

    pub fn collect_to_vec(self) -> Vec<ParsedRubyTxtElement> {
        // String を纏める
        let mut items = Vec::new();
        for item in self.items {
            if let ParsedRubyTxtElement::String { value } = &item {
                if let Some(ParsedRubyTxtElement::String { value: last_value }) = items.last_mut() {
                    last_value.push_str(&value);
                    continue;
                }
            }

            items.push(item);
        }

        items
    }
}

impl<Idx> std::ops::Index<Idx> for ParsedRubyTxtElementList
where
    Idx: std::slice::SliceIndex<[ParsedRubyTxtElement], Output = ParsedRubyTxtElement>,
{
    type Output = ParsedRubyTxtElement;

    #[inline(always)]
    fn index(&self, index: Idx) -> &Self::Output {
        self.items.index(index)
    }
}
