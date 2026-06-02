use anyflipdl_lib::services::metadata_fetcher;

fn main() {
    let body = std::fs::read_to_string("C:\\Users\\DivineOwl\\AppData\\Local\\Temp\\opencode\\live_config.js").unwrap();
    println!("body length: {}", body.len());

    // We can't easily call parse_config_js since it's private.
    // Instead, do what parse_config_js does: match the first {...} and parse JSON.
    let re = regex::Regex::new(r"(?s)\{.*\}").unwrap();
    let m = re.find(&body).unwrap();
    let json_str = m.as_str();
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    println!("has bookConfig: {}", value.get("bookConfig").is_some());
    println!("has meta: {}", value.get("meta").is_some());
    if let Some(meta) = value.get("meta") {
        println!("meta.title: {:?}", meta.get("title"));
        println!("meta.pageCount: {:?}", meta.get("pageCount"));
    }
    println!("has fliphtml5_pages: {}", value.get("fliphtml5_pages").is_some());
    if let Some(pages) = value.get("fliphtml5_pages").and_then(|v| v.as_array()) {
        println!("fliphtml5_pages count: {}", pages.len());
        println!("first page: {}", pages[0]);
    }
}
