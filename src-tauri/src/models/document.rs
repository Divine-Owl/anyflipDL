use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMetadata {
    pub url: String,
    pub title: String,
    pub page_count: usize,
    pub page_filenames: Vec<String>,
    #[serde(default)]
    pub author: Option<String>,
    /// Document publication or creation date as returned by Anyflip
    /// (`date`, `createDate`, `bookCreateDate`, or `publishDate`, first non-null).
    /// Raw string — frontend formats for display.
    #[serde(default)]
    pub date: Option<String>,
    /// URL to the document's cover image, derived as `{base_url}/files/large/1.jpg`.
    /// Frontend uses `<img onerror>` to fall back to a book icon.
    #[serde(default)]
    pub thumbnail: Option<String>,
    /// `true` if the document required a password to fetch metadata.
    /// Set by `fetch_with_password`; always `false` from the unprotected path.
    #[serde(default)]
    pub password_protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentResult {
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub page_count: Option<usize>,
    pub author: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> DocumentMetadata {
        DocumentMetadata {
            url: "https://online.anyflip.com/jjyfm/ikur/".to_string(),
            title: "Monogatari Series Volume 1".to_string(),
            page_count: 308,
            page_filenames: vec!["../files/mobile/1.webp".to_string()],
            author: Some("jjyfm".to_string()),
            date: None,
            thumbnail: Some(
                "https://online.anyflip.com/jjyfm/ikur/files/large/1.jpg".to_string(),
            ),
            password_protected: false,
        }
    }

    #[test]
    fn serializes_with_camel_case_fields() {
        let json = serde_json::to_string(&fixture()).unwrap();
        assert!(json.contains(r#""pageCount":308"#), "got: {json}");
        assert!(json.contains(r#""pageFilenames":["#), "got: {json}");
        assert!(json.contains(r#""passwordProtected":false"#), "got: {json}");
        assert!(!json.contains("page_count"), "snake_case leaked: {json}");
        assert!(!json.contains("page_filenames"), "snake_case leaked: {json}");
        assert!(!json.contains("password_protected"), "snake_case leaked: {json}");
    }

    #[test]
    fn deserializes_from_camel_case() {
        let json = r#"{
            "url": "https://online.anyflip.com/jjyfm/ikur/",
            "title": "X",
            "pageCount": 12,
            "pageFilenames": ["1.webp"],
            "passwordProtected": false
        }"#;
        let meta: DocumentMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.page_count, 12);
        assert_eq!(meta.page_filenames, vec!["1.webp"]);
        assert!(!meta.password_protected);
    }
}
