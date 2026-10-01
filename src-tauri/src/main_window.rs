//! The main window, built in code so it can route links and downloads from the
//! embedded CLI web UI: the webview's defaults drop `target=_blank` links and
//! cancel `<a download>` clicks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::webview::{DownloadEvent, NewWindowResponse};
use tauri::{AppHandle, Manager, Url, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

const MAIN_WINDOW_LABEL: &str = "main";

/// Build the main window from its `tauri.conf.json` entry (marked `"create": false`).
pub fn build(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == MAIN_WINDOW_LABEL)
        .ok_or("main window config not found")?
        .clone();

    let handle = app.handle().clone();
    let new_window_handle = handle.clone();
    let navigation_handle = handle.clone();
    // Destination per download URL; macOS doesn't report the path on completion.
    let destinations = Mutex::new(HashMap::<Url, PathBuf>::new());

    WebviewWindowBuilder::from_config(&handle, &config)?
        .on_new_window(move |url, _features| {
            if is_web_link(&url) {
                open_in_browser(&new_window_handle, &url);
            } else {
                log::warn!("Blocked new window for {url}");
            }
            NewWindowResponse::Deny
        })
        .on_navigation(move |url| {
            if is_web_link(url) && !is_local(url) {
                open_in_browser(&navigation_handle, url);
                return false;
            }
            true
        })
        .on_download(move |webview, event| {
            match event {
                // Keep the default destination: the Downloads folder, without overwriting.
                DownloadEvent::Requested { url, destination } => {
                    log::info!("Downloading {url} to {}", destination.display());
                    if let Ok(mut map) = destinations.lock() {
                        map.insert(url, destination.clone());
                    }
                }
                DownloadEvent::Finished { url, path, success } => {
                    let saved = destinations
                        .lock()
                        .ok()
                        .and_then(|mut map| map.remove(&url));
                    let (title, body) = download_notification(path.or(saved).as_deref(), success);
                    let _ = webview
                        .app_handle()
                        .notification()
                        .builder()
                        .title(title)
                        .body(body)
                        .show();
                }
                _ => {}
            }
            true
        })
        .build()?;
    Ok(())
}

fn open_in_browser(app: &AppHandle, url: &Url) {
    if let Err(e) = app.opener().open_url(url.as_str(), None::<&str>) {
        log::warn!("Failed to open {url}: {e}");
    }
}

/// Links meant for the system browser or mail client.
fn is_web_link(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https" | "mailto")
}

/// The app's own pages and the local CLI server, which stay inside the window.
fn is_local(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("localhost" | "127.0.0.1" | "[::1]" | "tauri.localhost")
    )
}

/// Notification title and body for a finished download.
fn download_notification(path: Option<&Path>, success: bool) -> (&'static str, String) {
    let file_name = path
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned());
    let folder = path
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned());
    match (success, file_name, folder) {
        (true, Some(file), Some(folder)) => {
            ("Download complete", format!("Saved {file} to {folder}"))
        }
        (true, _, _) => (
            "Download complete",
            "Saved to your Downloads folder".to_string(),
        ),
        (false, Some(file), _) => ("Download failed", format!("Could not save {file}")),
        (false, None, _) => ("Download failed", "Could not save the file".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unwrap_used)]
    fn url(s: &str) -> Url {
        s.parse().unwrap()
    }

    #[test]
    fn external_links_are_web_links() {
        assert!(is_web_link(&url("https://github.com/runkids/skillshare")));
        assert!(is_web_link(&url("mailto:someone@example.com")));
    }

    #[test]
    fn blob_and_app_urls_are_not_web_links() {
        assert!(!is_web_link(&url("blob:http://localhost:19420/abc")));
        assert!(!is_web_link(&url("tauri://localhost/index.html")));
        assert!(!is_web_link(&url("about:blank")));
    }

    #[test]
    fn cli_server_and_app_urls_are_local() {
        assert!(is_local(&url("http://localhost:19420/skills")));
        assert!(is_local(&url("http://127.0.0.1:19420/")));
        assert!(is_local(&url("http://[::1]:19420/")));
        assert!(is_local(&url("http://tauri.localhost/")));
        assert!(is_local(&url("tauri://localhost/")));
    }

    #[test]
    fn remote_hosts_are_not_local() {
        assert!(!is_local(&url("https://github.com/runkids/skillshare")));
        assert!(!is_local(&url("https://localhost.example.com/")));
        assert!(!is_local(&url("mailto:someone@example.com")));
    }

    #[test]
    fn finished_download_names_file_and_folder() {
        let path = Path::new("/Users/me/Downloads/skillshare-hub (1).json");
        assert_eq!(
            download_notification(Some(path), true),
            (
                "Download complete",
                "Saved skillshare-hub (1).json to Downloads".to_string()
            )
        );
    }

    #[test]
    fn finished_download_without_path_falls_back_to_downloads_folder() {
        assert_eq!(
            download_notification(None, true).1,
            "Saved to your Downloads folder"
        );
    }

    #[test]
    fn failed_download_says_so() {
        let path = Path::new("/Users/me/Downloads/skillshare-hub.json");
        assert_eq!(
            download_notification(Some(path), false),
            (
                "Download failed",
                "Could not save skillshare-hub.json".to_string()
            )
        );
    }
}
