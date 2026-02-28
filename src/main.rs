use epub::doc::EpubDoc;
use headless_chrome::{Browser, LaunchOptions, types::PrintToPdfOptions};
use regex::Regex;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;
use std::ffi::OsStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print!("Enter the path to your EPUB file: ");
    io::stdout().flush()?; 
    
    let mut input_path = String::new();
    io::stdin().read_line(&mut input_path)?;
    let input_path = input_path.trim().trim_matches('"').to_string();
    let output_path_pdf = input_path.replace(".epub", ".pdf");
    let output_path_html = input_path.replace(".epub", ".html");

    let mut doc = EpubDoc::new(&input_path)?;
    
    // 1. Setup Assets
    let asset_dir = Path::new("epub_assets");
    if asset_dir.exists() { fs::remove_dir_all(asset_dir)?; }
    fs::create_dir(asset_dir)?;

    println!("Extracting images...");
    let resource_ids: Vec<String> = doc.resources.keys().cloned().collect();
    for id in resource_ids {
        let mime = doc.resources.get(&id).unwrap().mime.clone();
        if mime.starts_with("image/") {
            if let Some((data, _)) = doc.get_resource(&id) {
                let path_str = doc.resources.get(&id).unwrap().path.to_string_lossy();
                let out_path = asset_dir.join(path_str.as_ref());
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(out_path, data)?;
            }
        }
    }

    // 2. Build HTML
    let mut toc_html = String::from("<div id='toc' style='page-break-after: always;'><h1>Table of Contents</h1><ul>");
    for (i, nav) in doc.toc.iter().enumerate() {
        toc_html.push_str(&format!("<li><a href='#chap-{}'>{}</a></li>", i, nav.label));
    }
    toc_html.push_str("</ul></div>");

    let custom_style = "<style>
        body { font-family: 'Georgia', serif; font-size: 11pt; line-height: 1.6; margin: 0.5in; }
        img { max-width: 100%; height: auto; display: block; margin: 1em auto; }
        h1, h2 { break-before: page; }
        p { text-align: justify; margin-bottom: 1em; }
    </style>";

    let mut full_html = format!("<!DOCTYPE html><html><head><meta charset='utf-8'>{}</head><body>{}", custom_style, toc_html);
    let re_body = Regex::new(r"(?is)<body.*?>(.*?)</body>").unwrap();

    println!("Merging chapters...");
    for i in 0..doc.get_num_chapters() {
        doc.set_current_chapter(i);
        if let Some((content, _)) = doc.get_current() {
            let text = String::from_utf8_lossy(&content);
            let cleaned = text.replace("src=\"", "src=\"epub_assets/").replace("src='", "src='epub_assets/");
            if let Some(caps) = re_body.captures(&cleaned) {
                full_html.push_str(&format!("<div id='chap-{}'>{}</div>", i, &caps[1]));
            }
        }
    }
    full_html.push_str("</body></html>");
    fs::write(&output_path_html, &full_html)?;

    // 3. Launch with Microsoft Edge and High Timeout
    println!("Launching Microsoft Edge...");
    
    // Common Windows path for Edge. Change this if your Edge is installed elsewhere.
    let edge_path = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe";

    let browser = Browser::new(
        LaunchOptions::default_builder()
            .path(Some(edge_path.into())) // Pointing to Edge
            .headless(true)
            .args(vec![
                OsStr::new("--disable-gpu"),
                OsStr::new("--no-sandbox"),
                OsStr::new("--disable-dev-shm-usage"),
                OsStr::new("--memory-pressure-off"),
            ])
            // This tells Rust to wait much longer for the browser to respond
            .idle_browser_timeout(Duration::from_secs(120)) 
            .build()
            .unwrap()
    )?;

    let tab = browser.new_tab()?;
    let absolute_path = fs::canonicalize(&output_path_html)?;
    let url = format!("file:///{}", absolute_path.to_string_lossy().replace("\\", "/").trim_start_matches("//?/"));
    
    tab.navigate_to(&url)?;
    
    println!("Laying out 553 pages... this may take up to a minute.");
    // Wait for the browser to finish its internal 'painting'
    std::thread::sleep(Duration::from_secs(60)); 

    let pdf_options = PrintToPdfOptions {
        display_header_footer: Some(true),
        footer_template: Some("<div style='font-size: 10px; width: 100%; text-align: center;'>Page <span class='pageNumber'></span> of <span class='totalPages'></span></div>".to_string()),
        header_template: Some("<div></div>".to_string()),
        print_background: Some(true),
        margin_top: Some(0.5),
        margin_bottom: Some(0.75),
        ..Default::default()
    };

    // Use a longer timeout for the print command itself
    let pdf_data = tab.print_to_pdf(Some(pdf_options))?;
    fs::write(&output_path_pdf, pdf_data)?;

    println!("\nSuccess! Generated PDF and HTML using Edge.");
    Ok(())
}