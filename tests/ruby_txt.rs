use std::fs;

use anyhow::Result;

use aozorabunko_json::ruby_txt::{
    parser::parse_ruby_txt, renderer::render_ruby_txt, tokenizer::tokenize_ruby_txt,
};

static RUBY_TXT_SUFFIX: &str = ".ruby.txt";

#[test]
fn test_ruby_txt_all() -> Result<()> {
    let paths = fs::read_dir("./tests").unwrap();
    for path in paths {
        let path = path.unwrap().path();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        if !file_name.ends_with(RUBY_TXT_SUFFIX) {
            continue;
        }

        let file_stem = &file_name[..(file_name.len() - RUBY_TXT_SUFFIX.len())];

        let txt = fs::read_to_string(&path).unwrap();

        let content = tokenize_ruby_txt(&txt)?;

        let content = parse_ruby_txt(&content)?;
        fs::write(
            path.with_file_name(format!("{}_parsed.json", file_stem)),
            serde_json::to_string_pretty(&content)?,
        )?;

        let content = render_ruby_txt(&content)?;
        fs::write(
            path.with_file_name(format!("{}_rendered.json", file_stem)),
            serde_json::to_string_pretty(&content)?,
        )?;
    }

    Ok(())
}
