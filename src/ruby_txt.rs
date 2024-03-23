// 青空文庫 注記一覧 https://www.aozora.gr.jp/annotation/（2010 年 4 月 1 日公布）のフォーマットに従った解析
//
// フォーマットから外れたものは基本的にエラーとするが，一部フールプルーフする：
// - 改行は公式に CR+LF とされているが完全には統一されていない
// - "底本：" は "底本:" でもよい
// - 長いハイフンは "テキスト中に現れる記号について" を示すためとされているが
//   単なる区切り？としての利用もある
//   - (例) https://www.aozora.gr.jp/cards/000124/card652.html

mod annotation_parser;
mod block_parser;
mod gaiji_accent_decomposition_parser;
mod gaiji_annotation_parser;
pub mod parser;
pub mod parser_helper;
mod ruby_parser;
pub mod tokenizer;
mod utility;
