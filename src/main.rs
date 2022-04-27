use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use log::{debug, error, info};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::ffi::OsStr;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;

lazy_static::lazy_static! {
    static ref HTML_INPUT_DIRECTORY : String = {
        let in_html = std::env::var("HTML_INPUT_DIRECTORY").unwrap_or_else(|_|"pages".into());
        std::fs::create_dir_all(&in_html).expect("could not create input dir");
        in_html
    };
    static ref PDF_OUTPUT_DIRECTORY : String = {
        let out_pdf = std::env::var("PDF_OUTPUT_DIRECTORY").unwrap_or_else(|_|"./pdf".into());
         std::fs::create_dir_all(&out_pdf).expect("could not create output dir");
         out_pdf
    };
}
fn watch(tab: Arc<Tab>) -> Result<(), failure::Error> {
    info!("Start watching  on '{}'", &*HTML_INPUT_DIRECTORY);
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))
        .map_err(|e| failure::format_err!("watch error {e}"))?;

    watcher
        .watch(&*HTML_INPUT_DIRECTORY, RecursiveMode::NonRecursive)
        .map_err(|e| failure::format_err!("watch error {e}"))?;

    info!(
        "Started. Ready to receive files from '{}'",
        &*HTML_INPUT_DIRECTORY
    );

    loop {
        match rx.recv() {
            Ok(notify::DebouncedEvent::Rename(source, destination)) => {
                debug!("rename event: {source:?}, {destination:?}");
                if destination
                    .extension()
                    .and_then(OsStr::to_str)
                    .filter(|ex| ex.ends_with("html"))
                    .is_some()
                {
                    let path = destination.canonicalize()?;
                    let (path, file_name, page) = {
                        let file_name = source
                            .file_name()
                            .and_then(|f| f.to_str())
                            .ok_or("no filename")
                            .map_err(|e| failure::format_err!("error: {e}"))?;

                        if let Some(path) = path.to_str() {
                            Ok((path.to_string(), file_name, format!("file://{path}")))
                        } else {
                            Err(failure::format_err!("path not found"))
                        }
                    }?;
                    let pdf = tab
                        .navigate_to(&page)?
                        .wait_until_navigated()?
                        .print_to_pdf(Default::default())?;

                    let new_path = format!(
                        "{}/{}",
                        &*PDF_OUTPUT_DIRECTORY,
                        file_name.replace(".html", ".pdf")
                    );
                    debug!("{new_path}");
                    std::fs::write(Path::new(&new_path), pdf)?;

                    let new_path = format!(
                        "{}/{}",
                        &*HTML_INPUT_DIRECTORY,
                        file_name.replace(".html", ".html.processed")
                    );
                    std::fs::rename(path, new_path)?;
                }
            }
            Ok(notify::DebouncedEvent::Create(event)) => {
                debug!("create event: {event:?}");

                if event
                    .extension()
                    .and_then(OsStr::to_str)
                    .filter(|ex| ex.ends_with("html"))
                    .is_some()
                {
                    let path = event.canonicalize()?;
                    let path = path
                        .to_str()
                        .ok_or("path cannot be converted to string")
                        .map_err(|e| failure::format_err!("error: {e}"))?;
                    let new_name = format!("{}.html", uuid::Uuid::new_v4());
                    std::fs::rename(path, format!("{}/{new_name}", &*HTML_INPUT_DIRECTORY,))?;
                }
            }

            Ok(event) => info!("watch event:  {event:?}"),
            Err(e) => error!("watch error: {:?}", e),
        }
    }
}

fn main() -> Result<(), failure::Error> {
    env_logger::init();
    let options = LaunchOptionsBuilder::default()
        .sandbox(false)
        .idle_browser_timeout(Duration::MAX)
        .build()
        .map_err(|e| failure::format_err!("invalid options: {e}"))?;

    let browser = Browser::new(options)?;
    let tab = browser.wait_for_initial_tab()?;

    watch(tab)
}
