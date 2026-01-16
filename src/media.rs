//! Media extraction and downloading for GitHub issue descriptions.
//!
//! Parses issue bodies for image and video URLs, downloads them locally,
//! and rewrites the body to reference local files so Claude can view them.

use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Type of media found in issue description
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Image,
    Video,
}

/// A media item extracted from an issue body
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MediaItem {
    /// The original URL of the media
    pub url: String,
    /// Type of media (image or video)
    pub media_type: MediaType,
    /// Alt text if available (from markdown or img alt attribute)
    pub alt_text: Option<String>,
    /// The full original match string (for replacement)
    pub original_match: String,
}

/// Extract all media URLs from an issue body.
///
/// Supports:
/// - HTML `<img src="...">` and `<img src='...'>`
/// - HTML `<video src="...">` and `<video src='...'>`
/// - Markdown `![alt](url)`
pub fn extract_media_urls(body: &str) -> Vec<MediaItem> {
    let mut items = Vec::new();

    // HTML img tags: Extract src and optionally alt from any order of attributes
    // First, find all img tags
    let img_tag_regex = Regex::new(r#"<img\s+[^>]*?/?>"#).unwrap();
    let src_regex = Regex::new(r#"src\s*=\s*["']([^"']+)["']"#).unwrap();
    let alt_regex = Regex::new(r#"alt\s*=\s*["']([^"']*)["']"#).unwrap();

    for tag_match in img_tag_regex.find_iter(body) {
        let tag = tag_match.as_str();
        // Extract src attribute
        if let Some(src_cap) = src_regex.captures(tag) {
            let url = src_cap.get(1).unwrap().as_str().to_string();
            // Extract alt attribute (if present)
            let alt_text = alt_regex
                .captures(tag)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());

            items.push(MediaItem {
                url,
                media_type: MediaType::Image,
                alt_text,
                original_match: tag.to_string(),
            });
        }
    }

    // HTML video tags: <video src="...">
    let video_regex = Regex::new(r#"<video\s+[^>]*?src\s*=\s*["']([^"']+)["'][^>]*?/?>"#).unwrap();

    for cap in video_regex.captures_iter(body) {
        let url = cap.get(1).map(|m| m.as_str().to_string()).unwrap();
        items.push(MediaItem {
            url,
            media_type: MediaType::Video,
            alt_text: None,
            original_match: cap.get(0).unwrap().as_str().to_string(),
        });
    }

    // Markdown images: ![alt](url)
    let md_img_regex = Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)").unwrap();

    for cap in md_img_regex.captures_iter(body) {
        let alt_text = cap.get(1).map(|m| m.as_str().to_string());
        let url = cap.get(2).map(|m| m.as_str().to_string()).unwrap();

        // Determine type from URL extension
        let media_type = if is_video_url(&url) {
            MediaType::Video
        } else {
            MediaType::Image
        };

        // Avoid duplicates (same URL might appear in both HTML and markdown)
        if !items.iter().any(|i| i.url == url) {
            items.push(MediaItem {
                url,
                media_type,
                alt_text,
                original_match: cap.get(0).unwrap().as_str().to_string(),
            });
        }
    }

    items
}

/// Check if a URL points to a video file based on extension
fn is_video_url(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    url_lower.ends_with(".mp4")
        || url_lower.ends_with(".webm")
        || url_lower.ends_with(".mov")
        || url_lower.ends_with(".avi")
        || url_lower.ends_with(".mkv")
        || url_lower.contains(".mp4?")
        || url_lower.contains(".webm?")
        || url_lower.contains(".mov?")
}

/// Get file extension from URL or content type
fn get_extension(url: &str, content_type: Option<&str>) -> String {
    // Try to get from content type first
    if let Some(ct) = content_type {
        let ext = match ct {
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            "video/mp4" => "mp4",
            "video/webm" => "webm",
            "video/quicktime" => "mov",
            _ => "",
        };
        if !ext.is_empty() {
            return ext.to_string();
        }
    }

    // Fall back to URL extension
    let url_path = url.split('?').next().unwrap_or(url);
    if let Some(ext) = url_path.rsplit('.').next() {
        let ext = ext.to_lowercase();
        if matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "mp4" | "webm" | "mov" | "avi" | "mkv"
        ) {
            return ext;
        }
    }

    // Default based on likely type
    "png".to_string()
}

/// Download a media item to the destination directory.
///
/// Returns the local path where the file was saved.
pub async fn download_media(
    client: &reqwest::Client,
    item: &MediaItem,
    dest_dir: &Path,
    index: usize,
) -> Result<PathBuf> {
    let response = client
        .get(&item.url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch media from {}", item.url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download media from {}: HTTP {}",
            item.url,
            response.status()
        );
    }

    // Get content type for extension detection
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or(s).trim());

    let ext = get_extension(&item.url, content_type);
    let prefix = match item.media_type {
        MediaType::Image => "image",
        MediaType::Video => "video",
    };
    let filename = format!("{}-{}.{}", prefix, index, ext);
    let dest_path = dest_dir.join(&filename);

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("Failed to read media bytes from {}", item.url))?;

    std::fs::write(&dest_path, &bytes)
        .with_context(|| format!("Failed to write media to {}", dest_path.display()))?;

    tracing::debug!(
        "Downloaded {} to {}",
        item.url,
        dest_path.display()
    );

    Ok(dest_path)
}

/// Process an issue body: extract media URLs, download them, and rewrite the body.
///
/// Images are replaced with local file paths.
/// Videos are replaced with local file paths plus a note that Claude can't read them.
///
/// Note: For GitHub issues, prefer `process_issue_body_with_html` which handles
/// signed URLs for private attachments.
#[allow(dead_code)]
pub async fn process_issue_body(
    body: &str,
    dest_dir: &Path,
    client: &reqwest::Client,
) -> Result<String> {
    let items = extract_media_urls(body);

    if items.is_empty() {
        return Ok(body.to_string());
    }

    tracing::info!("Found {} media items in issue body", items.len());

    let mut processed_body = body.to_string();

    for (index, item) in items.iter().enumerate() {
        match download_media(client, item, dest_dir, index).await {
            Ok(local_path) => {
                // Create replacement text
                let replacement = match item.media_type {
                    MediaType::Image => {
                        // Just the local path - Claude can read it
                        local_path.display().to_string()
                    }
                    MediaType::Video => {
                        // Local path with note about Claude's limitation
                        format!(
                            "{} [Video - not readable by Claude]",
                            local_path.display()
                        )
                    }
                };

                processed_body = processed_body.replace(&item.original_match, &replacement);
                tracing::info!(
                    "Replaced {} with local path {}",
                    item.url,
                    local_path.display()
                );
            }
            Err(e) => {
                // Keep original on failure
                tracing::warn!("Failed to download {}: {}. Keeping original URL.", item.url, e);
            }
        }
    }

    Ok(processed_body)
}

/// Extract the asset ID from a GitHub user-attachments URL.
///
/// Examples:
/// - `https://github.com/user-attachments/assets/6ad6bd37-7044-4a5d-8c74-cb7576e415c2`
/// - `https://private-user-images.githubusercontent.com/.../535780376-6ad6bd37-7044-4a5d-8c74-cb7576e415c2.png?jwt=...`
///
/// Returns the UUID portion (e.g., `6ad6bd37-7044-4a5d-8c74-cb7576e415c2`)
fn extract_asset_id(url: &str) -> Option<String> {
    // Pattern: UUID format (8-4-4-4-12 hex chars)
    let uuid_regex = Regex::new(r"([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})")
        .unwrap();

    uuid_regex
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Process an issue body using signed URLs from body_html.
///
/// GitHub user-attachments (images uploaded via the web UI) require special handling:
/// - The raw body contains URLs like `https://github.com/user-attachments/assets/UUID`
/// - These URLs return 404 when accessed with API tokens
/// - The body_html (fetched with `Accept: application/vnd.github.full+json`) contains
///   signed URLs with JWT tokens that can be downloaded
///
/// This function:
/// 1. Extracts media from body_html (which has signed URLs)
/// 2. Downloads using those signed URLs
/// 3. Rewrites the original body with local file paths
pub async fn process_issue_body_with_html(
    body: &str,
    body_html: &str,
    dest_dir: &Path,
    client: &reqwest::Client,
) -> Result<String> {
    // Extract media from body_html (has signed URLs we can actually download)
    let html_items = extract_media_urls(body_html);

    if html_items.is_empty() {
        return Ok(body.to_string());
    }

    tracing::info!(
        "Found {} media items in body_html with signed URLs",
        html_items.len()
    );

    // Extract media from original body (has URLs we need to replace)
    let body_items = extract_media_urls(body);

    // Build a map from asset ID to signed URL
    let mut signed_urls: std::collections::HashMap<String, &MediaItem> =
        std::collections::HashMap::new();
    for item in &html_items {
        if let Some(asset_id) = extract_asset_id(&item.url) {
            signed_urls.insert(asset_id, item);
        }
    }

    let mut processed_body = body.to_string();
    let mut download_index = 0;

    for body_item in &body_items {
        // Try to find the signed URL for this asset
        let download_item = if let Some(asset_id) = extract_asset_id(&body_item.url) {
            signed_urls.get(&asset_id).copied().unwrap_or(body_item)
        } else {
            body_item
        };

        // Download using the signed URL (or original if no signed URL found)
        match download_media(client, download_item, dest_dir, download_index).await {
            Ok(local_path) => {
                let replacement = match body_item.media_type {
                    MediaType::Image => local_path.display().to_string(),
                    MediaType::Video => {
                        format!("{} [Video - not readable by Claude]", local_path.display())
                    }
                };

                processed_body = processed_body.replace(&body_item.original_match, &replacement);
                tracing::info!(
                    "Downloaded {} -> {}",
                    body_item.url,
                    local_path.display()
                );
                download_index += 1;
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to download {}: {}. Keeping original URL.",
                    body_item.url,
                    e
                );
            }
        }
    }

    Ok(processed_body)
}

/// Create an HTTP client configured for GitHub asset downloads.
///
/// Note: For GitHub user-attachments (private repo images), the signed URLs
/// from body_html contain JWT tokens and don't need additional auth headers.
/// This client is kept simple intentionally.
pub fn create_media_client(_github_token: &str) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("pleb-media-downloader")
        .build()
        .context("Failed to create HTTP client for media downloads")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_html_img_double_quotes() {
        let body = r#"Some text <img src="https://example.com/image.png" alt="Test"> more text"#;
        let items = extract_media_urls(body);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url, "https://example.com/image.png");
        assert_eq!(items[0].media_type, MediaType::Image);
        assert_eq!(items[0].alt_text, Some("Test".to_string()));
    }

    #[test]
    fn test_extract_html_img_single_quotes() {
        let body = r#"<img src='https://example.com/image.jpg' />"#;
        let items = extract_media_urls(body);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url, "https://example.com/image.jpg");
        assert_eq!(items[0].alt_text, None);
    }

    #[test]
    fn test_extract_html_img_with_attributes() {
        let body = r#"<img width="800" height="600" src="https://github.com/user-attachments/assets/abc123.png" alt="Screenshot">"#;
        let items = extract_media_urls(body);

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].url,
            "https://github.com/user-attachments/assets/abc123.png"
        );
    }

    #[test]
    fn test_extract_html_video() {
        let body = r#"<video src="https://example.com/demo.mp4"></video>"#;
        let items = extract_media_urls(body);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url, "https://example.com/demo.mp4");
        assert_eq!(items[0].media_type, MediaType::Video);
    }

    #[test]
    fn test_extract_markdown_image() {
        let body = "Check out this ![screenshot](https://example.com/img.png) here";
        let items = extract_media_urls(body);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url, "https://example.com/img.png");
        assert_eq!(items[0].media_type, MediaType::Image);
        assert_eq!(items[0].alt_text, Some("screenshot".to_string()));
    }

    #[test]
    fn test_extract_markdown_video() {
        let body = "Demo: ![video](https://example.com/demo.mp4)";
        let items = extract_media_urls(body);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url, "https://example.com/demo.mp4");
        assert_eq!(items[0].media_type, MediaType::Video);
    }

    #[test]
    fn test_extract_multiple_items() {
        let body = r#"
            First image: ![img1](https://example.com/1.png)
            Second: <img src="https://example.com/2.jpg">
            Video: <video src="https://example.com/3.mp4"></video>
        "#;
        let items = extract_media_urls(body);

        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_extract_no_media() {
        let body = "Just some plain text without any images or videos.";
        let items = extract_media_urls(body);

        assert!(items.is_empty());
    }

    #[test]
    fn test_extract_github_user_attachments() {
        // Real format from GitHub issues
        let body = r#"<img width="1844" height="669" alt="Image" src="https://github.com/user-attachments/assets/6ad6bd37-7044-4a5d-8c74-cb7576e415c2" />"#;
        let items = extract_media_urls(body);

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].url,
            "https://github.com/user-attachments/assets/6ad6bd37-7044-4a5d-8c74-cb7576e415c2"
        );
        assert_eq!(items[0].alt_text, Some("Image".to_string()));
    }

    #[test]
    fn test_no_duplicate_urls() {
        // Same image in both markdown and HTML
        let body = r#"
            ![img](https://example.com/same.png)
            <img src="https://example.com/same.png">
        "#;
        let items = extract_media_urls(body);

        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_is_video_url() {
        assert!(is_video_url("https://example.com/video.mp4"));
        assert!(is_video_url("https://example.com/video.MP4"));
        assert!(is_video_url("https://example.com/video.webm"));
        assert!(is_video_url("https://example.com/video.mov"));
        assert!(is_video_url("https://example.com/video.mp4?token=abc"));
        assert!(!is_video_url("https://example.com/image.png"));
        assert!(!is_video_url("https://example.com/image.jpg"));
    }

    #[test]
    fn test_get_extension_from_content_type() {
        assert_eq!(get_extension("", Some("image/png")), "png");
        assert_eq!(get_extension("", Some("image/jpeg")), "jpg");
        assert_eq!(get_extension("", Some("video/mp4")), "mp4");
    }

    #[test]
    fn test_get_extension_from_url() {
        assert_eq!(get_extension("https://example.com/img.png", None), "png");
        assert_eq!(get_extension("https://example.com/img.PNG", None), "png");
        assert_eq!(
            get_extension("https://example.com/img.png?token=abc", None),
            "png"
        );
    }

    #[test]
    fn test_get_extension_default() {
        assert_eq!(
            get_extension("https://example.com/no-extension", None),
            "png"
        );
    }

    #[test]
    fn test_extract_asset_id_from_user_attachments() {
        let url = "https://github.com/user-attachments/assets/6ad6bd37-7044-4a5d-8c74-cb7576e415c2";
        assert_eq!(
            extract_asset_id(url),
            Some("6ad6bd37-7044-4a5d-8c74-cb7576e415c2".to_string())
        );
    }

    #[test]
    fn test_extract_asset_id_from_signed_url() {
        let url = "https://private-user-images.githubusercontent.com/812199/535780376-6ad6bd37-7044-4a5d-8c74-cb7576e415c2.png?jwt=eyJ...";
        assert_eq!(
            extract_asset_id(url),
            Some("6ad6bd37-7044-4a5d-8c74-cb7576e415c2".to_string())
        );
    }

    #[test]
    fn test_extract_asset_id_no_uuid() {
        let url = "https://example.com/image.png";
        assert_eq!(extract_asset_id(url), None);
    }
}
