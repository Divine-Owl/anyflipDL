#[tokio::main]
async fn main() {
    println!("Testing AnyflipDL download pipeline...");

    let url = "https://anyflip.com/qwqm/doui";

    // Test 1: Validate URL
    match anyflipdl_lib::services::validator::validate_url(url) {
        Ok(()) => println!("✓ URL validation passed"),
        Err(e) => {
            println!("✗ URL validation failed: {}", e);
            return;
        }
    }

    // Test 2: Parse URL
    let parsed = match anyflipdl_lib::services::url_parser::parse(url) {
        Ok(p) => {
            println!("✓ URL parsed: user={}, book={}", p.user, p.book);
            p
        }
        Err(e) => {
            println!("✗ URL parse failed: {}", e);
            return;
        }
    };

    // Test 3: Fetch metadata
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0")
        .build()
        .unwrap();

    let metadata = match anyflipdl_lib::services::metadata_fetcher::fetch(
        &client,
        &parsed,
    ).await {
        Ok(m) => {
            println!(
                "✓ Metadata fetched: title='{}', pages={}",
                m.title, m.page_count
            );
            m
        }
        Err(e) => {
            println!("✗ Metadata fetch failed: {}", e);
            return;
        }
    };

    if metadata.page_count == 0 {
        println!("✗ Zero pages detected");
        return;
    }

    // Test 4: Download first 2 pages
    let temp_dir = std::env::temp_dir().join("anyflipdl_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let pages = 2.min(metadata.page_count);
    println!("Downloading {} pages to {:?}...", pages, temp_dir);

    let mut downloaded = 0;
    for page in 1..=pages {
        let page_url = parsed.page_url(page);
        let page_path = temp_dir.join(format!("{}.webp", page));

        match client
            .get(&page_url)
            .header("Referer", format!("{}/", parsed.base_url))
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.bytes().await {
                        Ok(bytes) => {
                            std::fs::write(&page_path, &bytes).unwrap();
                            println!("  ✓ Page {}: {} bytes", page, bytes.len());
                            downloaded += 1;
                        }
                        Err(e) => println!("  ✗ Page {} read: {}", page, e),
                    }
                } else {
                    println!("  ✗ Page {} HTTP {}", page, resp.status());
                }
            }
            Err(e) => println!("  ✗ Page {} request: {}", page, e),
        }
    }

    if downloaded == 0 {
        println!("✗ No pages downloaded");
        let _ = std::fs::remove_dir_all(&temp_dir);
        return;
    }

    // Test 5: Generate PDF
    let output_path = std::env::temp_dir().join("anyflipdl_test_output.pdf");
    match anyflipdl_lib::services::pdf_generator::generate(
        &temp_dir,
        downloaded,
        &output_path,
        Some(&metadata),
    ).await {
        Ok(()) => {
            let size = std::fs::metadata(&output_path).unwrap().len();
            println!("✓ PDF generated: {} bytes → {:?}", size, output_path);
        }
        Err(e) => println!("✗ PDF failed: {}", e),
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::remove_file(&output_path);

    println!("\n✅ Download pipeline test complete!");
}
