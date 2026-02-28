use epub::doc::EpubDoc;
use headless_chrome::{Browser, LaunchOptions, types::PrintToPdfOptions};
use regex::Regex;
use std::{fs, thread, time::Duration};
use std::path::Path;
use std::ffi::OsStr;

// This macro brings in the AppWindow struct generated from appwindow.slint
slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    // CALLBACK: Open EPUB
    ui.on_open_epub_clicked({
        let ui_handle = ui_handle.clone();
        move || {
            let file = rfd::FileDialog::new()
                .add_filter("EPUB eBook", &["epub"])
                .pick_file();
            if let Some(path) = file {
                let ui = ui_handle.unwrap();
                ui.set_epub_path(path.to_string_lossy().to_string().into());
                ui.set_status_msg("File selected. Ready to convert.".into());
                ui.set_status_color(slint::Color::from_rgb_u8(0, 0, 0)); // Black
                
                if let Some(parent) = path.parent() {
                    ui.set_save_path(parent.to_string_lossy().to_string().into());
                }
            }
        }
    });

    // CALLBACK: Select Destination Folder
    ui.on_select_save_clicked({
        let ui_handle = ui_handle.clone();
        move || {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                ui_handle.unwrap().set_save_path(folder.to_string_lossy().to_string().into());
            }
        }
    });

    // CALLBACK: Transform
    ui.on_transform_clicked({
        let ui_handle = ui_handle.clone();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_is_busy(true);
            ui.set_progress(0.0);
            ui.set_status_msg("Processing... please wait.".into());
            ui.set_status_color(slint::Color::from_rgb_u8(0, 0, 255)); // Blue

            let epub_path = ui.get_epub_path().to_string();
            let dest_dir = ui.get_save_path().to_string();
            let do_pdf = ui.get_export_pdf();
            let do_html = ui.get_export_html();

            // Prepare output filenames based on EPUB name
            let file_stem = Path::new(&epub_path).file_stem().unwrap_or(OsStr::new("output")).to_string_lossy();
            let pdf_path = Path::new(&dest_dir).join(format!("{}.pdf", file_stem)).to_string_lossy().to_string();
            let html_path = Path::new(&dest_dir).join(format!("{}.html", file_stem)).to_string_lossy().to_string();

            let thread_handle = ui_handle.clone();
            thread::spawn(move || {
                match perform_conversion(epub_path, pdf_path, html_path, do_pdf, do_html, thread_handle.clone()) {
                    Ok(_) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = thread_handle.upgrade() {
                                ui.set_status_msg("Success! Conversion Complete.".into());
                                ui.set_status_color(slint::Color::from_rgb_u8(0, 128, 0)); // Green
                                ui.set_is_busy(false);
                                ui.set_progress(1.0);
                            }
                        });
                    }
                    Err(e) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = thread_handle.upgrade() {
                                ui.set_status_msg(format!("Error: {}", e).into());
                                ui.set_status_color(slint::Color::from_rgb_u8(200, 0, 0)); // Red
                                ui.set_is_busy(false);
                            }
                        });
                    }
                }
            });
        }
    });

    ui.run()?;
    Ok(())
}

fn perform_conversion(
    input: String, 
    output_pdf: String, 
    output_html: String,
    do_pdf: bool,
    do_html: bool,
    ui_handle: slint::Weak<AppWindow>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
    // Helper to update progress bar across the thread boundary
    let update_progress = |val: f32| {
        let h = ui_handle.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = h.upgrade() {
                ui.set_progress(val);
            }
        });
    };

    update_progress(0.1);

    let mut doc = EpubDoc::new(&input).map_err(|e| e.to_string())?;
    
    // 1. Asset Extraction
    let asset_dir = Path::new("epub_assets");
    if asset_dir.exists() { fs::remove_dir_all(asset_dir).ok(); }
    fs::create_dir(asset_dir).ok();

    let resource_ids: Vec<String> = doc.resources.keys().cloned().collect();
    for id in resource_ids {
        let (res_path, res_mime) = {
            let res = doc.resources.get(&id).unwrap();
            (res.path.clone(), res.mime.clone())
        };

        if res_mime.starts_with("image/") {
            if let Some((data, _)) = doc.get_resource(&id) {
                let out_path = asset_dir.join(&res_path);
                if let Some(parent) = out_path.parent() { fs::create_dir_all(parent).ok(); }
                fs::write(out_path, data).ok();
            }
        }
    }

    update_progress(0.3);

    // 2. Merge Chapters into HTML stream
    let mut full_html = String::from("<!DOCTYPE html><html><head><meta charset='utf-8'><style>body{font-family: 'Georgia', serif; font-size: 11pt; line-height: 1.6; margin: 0.5in;} img{max-width: 100%; height: auto; display: block; margin: 1em auto;} h1, h2 {break-before: page;} p {text-align: justify; margin-bottom: 1em;}</style></head><body>");
    let re_body = Regex::new(r"(?is)<body.*?>(.*?)</body>").unwrap();

    let total_chaps = doc.get_num_chapters();
    for i in 0..total_chaps {
        doc.set_current_chapter(i);
        if let Some((content, _)) = doc.get_current() {
            let text = String::from_utf8_lossy(&content)
                .replace("src=\"", "src=\"epub_assets/")
                .replace("src='", "src='epub_assets/");
            if let Some(caps) = re_body.captures(&text) {
                full_html.push_str(&format!("<div id='chap-{}'>{}</div>", i, &caps[1]));
            }
        }
        // Progress bar covers the 30% to 60% range during merging
        update_progress(0.3 + (0.3 * (i as f32 / total_chaps as f32)));
    }
    full_html.push_str("</body></html>");

    // Write HTML to disk
    fs::write(&output_html, &full_html).ok();

    // 3. Conditional PDF Rendering via Headless Edge
    if do_pdf {
        update_progress(0.7);
        
        let edge_path = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe";
        let browser = Browser::new(
            LaunchOptions::default_builder()
                .path(Some(edge_path.into()))
                .headless(true)
                .args(vec![
                    OsStr::new("--disable-gpu"),
                    OsStr::new("--no-sandbox"),
                    OsStr::new("--disable-dev-shm-usage"),
                    OsStr::new("--memory-pressure-off")
                ])
                .idle_browser_timeout(Duration::from_secs(120))
                .build()
                .map_err(|e| e.to_string())?
        ).map_err(|e| e.to_string())?;

        let tab = browser.new_tab().map_err(|e| e.to_string())?;
        let abs_path = fs::canonicalize(&output_html).map_err(|e| e.to_string())?;
        let url = format!("file:///{}", abs_path.to_string_lossy().replace("\\", "/").trim_start_matches("//?/"));
        
        tab.navigate_to(&url).map_err(|e| e.to_string())?;
        
        // Wait for Chromium layout engine to stabilize for large documents
        thread::sleep(Duration::from_secs(45));
        update_progress(0.9);

        let pdf_options = PrintToPdfOptions {
            display_header_footer: Some(true),
            footer_template: Some("<div style='font-size: 10px; width: 100%; text-align: center;'>Page <span class='pageNumber'></span> of <span class='totalPages'></span></div>".to_string()),
            print_background: Some(true),
            margin_top: Some(0.5),
            margin_bottom: Some(0.75),
            ..Default::default()
        };

        let pdf_data = tab.print_to_pdf(Some(pdf_options)).map_err(|e| e.to_string())?;
        fs::write(&output_pdf, pdf_data).ok();
    }

    // 4. Cleanup Logic
    // If the user only wanted a PDF, remove the intermediate HTML file
    if !do_html {
        fs::remove_file(&output_html).ok();
    }

    Ok(())
}