use once_cell::sync::Lazy;
use std::collections::HashMap;

pub static JIS_X_0213: Lazy<HashMap<(usize, usize, usize), String>> = Lazy::new(|| {
    let json = include_str!("JIS_X_0213.json/JIS_X_0213.json");
    let json: serde_json::Value = serde_json::from_str(json).unwrap();
    json.as_array()
        .unwrap()
        .iter()
        .map(|item| {
            let plane = item.get("plane").unwrap().as_u64().unwrap() as usize;
            let row = item.get("row").unwrap().as_u64().unwrap() as usize;
            let cell = item.get("cell").unwrap().as_u64().unwrap() as usize;
            let char = item.get("char").unwrap().as_str().unwrap().to_owned();
            ((plane, row, cell), char)
        })
        .collect()
});
