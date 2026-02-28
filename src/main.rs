use epub::doc::EpubDoc;
use headless_chrome::{Browser, LaunchOptions, types::PrintToPdfOptions};
use regex::Regex;
use std::{fs, thread, time::Duration};
use std::path::Path; 
use std::ffi::OsStr;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    ui.on_open_epub_clicked({
        let ui_handle = ui_handle.clone();
        move || {
            let file = rfd::FileDialog::new()
                .add_filter("EPUB eBook", &["epub"])
                .pick_file();
            if let Some(path) = file {
                let ui = ui_handle.unwrap();
                ui.set_epub_path(path.to_string_lossy().to_string().into());
                
                let mut save_p = path.clone();
                save_p.set_extension("pdf");
                ui.set_save_path(save_p.to_string_lossy().to_string().into());
            }
        }
    });

    ui.on_select_save_clicked({
        let ui_handle = ui_handle.clone();
        move || {
            let file = rfd::FileDialog::new()
                .add_filter("PDF Document", &["pdf"])
                .save_file();
            if let Some(path) = file {
                ui_handle.unwrap().set_save_path(path.to_string_lossy().to_string().into());
            }
        }
    });

    ui.on_transform_clicked({
        let ui_handle = ui_handle.clone();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_is_busy(true);
            ui.set_progress(0.0);

            let epub_path = ui.get_epub_path().to_string();
            let pdf_path = ui.get_save_path().to_string();
            let html_path = epub_path.replace(".epub", ".html");

            // FIX E0507: Clone the handle specifically for the thread move
            let thread_handle = ui_handle.clone();
            thread::spawn(move || {
                if let Err(e) = perform_conversion(epub_path, pdf_path, html_path, thread_handle.clone()) {
                    eprintln!("Conversion error: {}", e);
                }
                
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = thread_handle.upgrade() {
                        ui.set_is_busy(false);
                        ui.set_progress(1.0);
                    }
                });
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
    ui_handle: slint::Weak<AppWindow>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
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
    
    let asset_dir = Path::new("epub_assets");
    if asset_dir.exists() { fs::remove_dir_all(asset_dir).ok(); }
    fs::create_dir(asset_dir).ok();

    let resource_ids: Vec<String> = doc.resources.keys().cloned().collect();
    for id in resource_ids {
        // FIX E0502: Clone the path and mime so the immutable borrow of 'doc' ends here
        let (res_path, res_mime) = {
            let res = doc.resources.get(&id).unwrap();
            (res.path.clone(), res.mime.clone())
        };

        if res_mime.starts_with("image/") {
            // Now we can safely borrow 'doc' mutably because 'res' is no longer alive
            if let Some((data, _)) = doc.get_resource(&id) {
                let out_path = asset_dir.join(&res_path);
                if let Some(parent) = out_path.parent() { fs::create_dir_all(parent).ok(); }
                fs::write(out_path, data).ok();
            }
        }
    }

    update_progress(0.3);

    let mut full_html = String::from("<html><head><meta charset='utf-8'><style>body{font-family:serif;margin:0.5in;}img{max-width:100%;height:auto;}</style></head><body>");
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
        update_progress(0.3 + (0.3 * (i as f32 / total_chaps as f32)));
    }
    full_html.push_str("</body></html>");
    fs::write(&output_html, &full_html).ok();

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
    
    thread::sleep(Duration::from_secs(45));
    update_progress(0.9);

    let pdf_options = PrintToPdfOptions {
        display_header_footer: Some(true),
        footer_template: Some("<div style='font-size: 10px; width: 100%; text-align: center;'>Page <span class='pageNumber'></span> of <span class='totalPages'></span></div>".to_string()),
        print_background: Some(true),
        ..Default::default()
    };

    let pdf_data = tab.print_to_pdf(Some(pdf_options)).map_err(|e| e.to_string())?;
    fs::write(&output_pdf, pdf_data).ok();

    Ok(())
}