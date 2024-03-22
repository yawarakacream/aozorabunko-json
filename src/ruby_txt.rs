// 青空文庫 注記一覧 https://www.aozora.gr.jp/annotation/（2010 年 4 月 1 日公布）のフォーマットに従った解析
//
// フォーマットから外れたものは基本的にエラーとするが，一部フールプルーフする：
// - 改行は公式に CR+LF とされているが完全には統一されていない
// - "底本：" の "底本" と '：' の間に文字があってもよい
// - 長いハイフンは "テキスト中に現れる記号について" を示すためとされているが
//   単なる区切り？としての利用もある
//   - (例) https://www.aozora.gr.jp/cards/000124/card652.html

mod annotation;
mod delimiter_and_tokens;
mod gaiji_accent_decomposition;
mod gaiji_annotation;
mod ruby;
pub mod ruby_txt_parser;
pub mod ruby_txt_tokenizer;
