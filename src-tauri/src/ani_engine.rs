//! Initial native-Rust port of the search / episode / stream-resolution core
//! from ani-cli 5.1.6. This is a starting module, not a complete port of every
//! CLI feature. Keep GPL-3.0-or-later notices when distributing derived work.

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use regex::Regex;
use reqwest::{header, Client, Url};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const BASE_API: &str = "https://hianime.at";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
const XOR_KEY: &[u8] = b"otaku-embed-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimeResult {
    pub id: String,
    pub title: String,
    /// Cover image URL if the result card had one. The UI shows a placeholder otherwise.
    #[serde(default)]
    pub poster: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSource {
    pub quality: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedEpisode {
    pub sources: Vec<StreamSource>,
    pub subtitles: Vec<SubtitleTrack>,
    pub audio_url: Option<String>,
    pub referer: String,
    pub mal_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub label: Option<String>,
    pub language: Option<String>,
    pub url: String,
    pub is_default: bool,
}

pub struct AniEngine {
    client: Client,
}

impl AniEngine {
    pub fn new() -> Result<Self> {
        let mut default_headers = header::HeaderMap::new();
        default_headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(USER_AGENT),
        );
        let client = Client::builder()
            .default_headers(default_headers)
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .context("creating HTTP client")?;
        Ok(Self { client })
    }

    async fn get_text(&self, url: &str, referer: Option<&str>) -> Result<String> {
        let mut req = self.client.get(url);
        if let Some(referer) = referer {
            req = req.header(header::REFERER, referer);
        }
        let response = req.send().await.context("request failed")?;
        let status = response.status();
        let body = response.text().await.context("reading response body")?;
        if !status.is_success() && !body.to_lowercase().contains("just a moment") {
            bail!("HTTP {status} while requesting {url}");
        }
        if body.to_lowercase().contains("<title>just a moment") {
            bail!(
                "The source site returned a browser-check page; the request could not be parsed."
            );
        }
        Ok(body)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<AnimeResult>> {
        let url = format!("{BASE_API}/search?keyword={}", urlencoding::encode(query));
        let page = self.get_text(&url, None).await?;
        // Match ani-cli's own parsing strategy: cut off the top-10 sidebar before
        // parsing. The provider's results wrapper class can change, so don't depend
        // on `.film_list-search` being present in the HTML.
        let search_page = page.split("id=\"main-sidebar\"").next().unwrap_or(&page);
        let document = Html::parse_document(search_page);
        let result_selector = Selector::parse(".film-detail").unwrap();
        let title_selector = Selector::parse("h3.film-name a").unwrap();
        let img_selector = Selector::parse("img").unwrap();
        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        for result in document.select(&result_selector) {
            let Some(anchor) = result.select(&title_selector).next() else {
                continue;
            };
            let Some(href) = anchor.value().attr("href") else {
                continue;
            };
            let Some(id) = href.trim_end_matches('/').rsplit('/').next() else {
                continue;
            };
            let title = anchor
                .value()
                .attr("title")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| anchor.text().collect::<String>().trim().to_string());
            if !id.is_empty() && !title.is_empty() && seen_ids.insert(id.to_string()) {
                // The cover image sits next to `.film-detail` inside the same result card.
                let poster = result
                    .parent()
                    .and_then(scraper::ElementRef::wrap)
                    .and_then(|card| {
                        card.select(&img_selector)
                            .find_map(|img| poster_url(img.value()))
                    });
                results.push(AnimeResult {
                    id: id.to_string(),
                    title,
                    poster,
                });
            }
        }
        Ok(results)
    }

    pub async fn episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        // ani-cli strips any trailing season suffix after the last hyphen for this endpoint.
        let slug = anime_id.rsplit('-').next().unwrap_or(anime_id);
        let url = format!(
            "{BASE_API}/api/theme/episode/list/{}",
            urlencoding::encode(slug)
        );
        let response = self.get_text(&url, None).await?;

        // The endpoint has returned both raw HTML and JSON-wrapped/escaped HTML
        // over time. Decode either shape before asking the HTML parser to inspect
        // the .ep-item elements. A movie is expected to have one item here too.
        let mut html_candidates = Vec::new();
        if let Ok(json) = serde_json::from_str::<Value>(&response) {
            collect_html_strings(&json, &mut html_candidates);
        }
        html_candidates.push(response.replace("\\|", "|").replace("\\\"", "\""));

        let item_selector = Selector::parse(".ep-item").unwrap();
        let mut episodes = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for html in html_candidates {
            let document = Html::parse_fragment(&html);
            for item in document.select(&item_selector) {
                let Some(id) = item.value().attr("data-id") else {
                    continue;
                };
                let Some(number) = item.value().attr("data-number") else {
                    continue;
                };
                if !id.is_empty()
                    && id.chars().all(|c| c.is_ascii_digit())
                    && !number.trim().is_empty()
                    && seen.insert(id.to_string())
                {
                    episodes.push(Episode {
                        id: id.to_string(),
                        number: number.to_string(),
                    });
                }
            }
            if !episodes.is_empty() {
                break;
            }
        }
        if episodes.is_empty() {
            bail!("The provider returned an episode-list response, but AniGUI could not find any episode IDs. This can happen if the provider changed its response format or the selected title has no episode entry.");
        }
        Ok(episodes)
    }

    pub async fn resolve_episode(
        &self,
        anime_id: &str,
        episode: &Episode,
        dub: bool,
    ) -> Result<ResolvedEpisode> {
        let _ = anime_id; // kept for API symmetry; the episode id is all the provider needs
        let servers_url = format!(
            "{BASE_API}/api/theme/episode/servers?episodeId={}",
            urlencoding::encode(&episode.id)
        );
        let servers_page = self.get_text(&servers_url, None).await?;
        // `scraper::Html` is not `Send`, so it must be dropped before the next `.await`.
        // Parsing inside its own block guarantees that.
        let embed_url = {
            let document = Html::parse_fragment(&servers_page.replace("\\\"", "\""));
            let server_selector = Selector::parse(".server-item").unwrap();
            let mut found = None;
            for server in document.select(&server_selector) {
                let data_type = server.value().attr("data-type").unwrap_or_default();
                let server_name = server.value().attr("data-server-name").unwrap_or_default();
                if data_type.eq_ignore_ascii_case(if dub { "dub" } else { "sub" })
                    && server_name.eq_ignore_ascii_case("ZokoAnime")
                {
                    if let Some(hash) = server.value().attr("data-hash") {
                        if let Ok(decoded) = STANDARD.decode(hash) {
                            if let Ok(url) = String::from_utf8(decoded) {
                                found = Some(url);
                                break;
                            }
                        }
                    }
                }
            }
            found
        };
        let embed_url = embed_url
            .ok_or_else(|| anyhow!("No supported ZokoAnime embed found for this audio mode"))?;
        let embed = Url::parse(&embed_url).context("invalid embed URL")?;
        let referer = format!(
            "{}://{}/",
            embed.scheme(),
            embed
                .host_str()
                .ok_or_else(|| anyhow!("embed URL has no host"))?
        );
        let mal_re = Regex::new(r"/mal/(\d+)/").unwrap();
        let mal_id = mal_re
            .captures(&embed_url)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        let embed_page = self.get_text(&embed_url, Some(&referer)).await?;
        let blob_re = Regex::new(r#"window\.__P="([^"]+)""#).unwrap();
        let blob = blob_re
            .captures(&embed_page)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| {
                anyhow!("Embed page did not contain its encoded player configuration")
            })?;
        let json_text = deobfuscate_blob(blob)?;
        let config: Value =
            serde_json::from_str(&json_text).context("decoded player config was not valid JSON")?;
        let master_url = find_m3u8(&config)
            .ok_or_else(|| anyhow!("No HLS playlist found in player configuration"))?;
        let playlist = self.get_text(&master_url, Some(&referer)).await?;
        let sources = parse_master_playlist(&master_url, &playlist)?;
        let subtitles = parse_subtitles(&config);
        let audio_url = parse_audio_url(&master_url, &playlist);
        Ok(ResolvedEpisode {
            sources,
            subtitles,
            audio_url,
            referer,
            mal_id,
        })
    }
}

/// Read an image URL from an <img>, preferring lazy-load attributes, and make it absolute.
fn poster_url(img: &scraper::node::Element) -> Option<String> {
    ["data-src", "data-original", "src"]
        .iter()
        .filter_map(|attr| img.attr(attr))
        .map(str::trim)
        .find(|value| !value.is_empty() && !value.starts_with("data:"))
        .map(|value| {
            if value.starts_with("//") {
                format!("https:{value}")
            } else if value.starts_with('/') {
                format!("{BASE_API}{value}")
            } else {
                value.to_string()
            }
        })
}

// Pull HTML-bearing strings out of common API response shapes (for example
// {"html":"..."} or nested data/result objects). The function deliberately
// keeps the response schema-agnostic so a provider field rename is less brittle.
fn collect_html_strings(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::String(text) if text.contains("ep-item") => {
            output.push(text.replace("\\|", "|").replace("\\\"", "\""));
        }
        Value::Object(map) => {
            for child in map.values() {
                collect_html_strings(child, output);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_html_strings(child, output);
            }
        }
        _ => {}
    }
}

fn deobfuscate_blob(blob: &str) -> Result<String> {
    let bytes = STANDARD
        .decode(blob)
        .context("player configuration was not valid base64")?;
    let decoded: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect();
    String::from_utf8(decoded).context("deobfuscated player config was not UTF-8")
}

fn find_m3u8(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(src) = map.get("src").and_then(Value::as_str) {
                if src.contains(".m3u8") {
                    return Some(src.to_string());
                }
            }
            map.values().find_map(find_m3u8)
        }
        Value::Array(items) => items.iter().find_map(find_m3u8),
        _ => None,
    }
}

fn parse_master_playlist(master_url: &str, playlist: &str) -> Result<Vec<StreamSource>> {
    let base = Url::parse(master_url).context("invalid master playlist URL")?;
    let lines: Vec<&str> = playlist.lines().map(str::trim).collect();
    let mut sources = Vec::new();
    let mut pending_quality: Option<String> = None;
    for line in lines {
        if line.starts_with("#EXT-X-STREAM-INF:") {
            let res_re = Regex::new(r"RESOLUTION=\d+x(\d+)").unwrap();
            let quality = res_re
                .captures(line)
                .and_then(|c| c.get(1))
                .map(|m| format!("{}p", m.as_str()))
                .or_else(|| {
                    Regex::new(r#"(?:^|,)NAME="?([^,"]+)"?"#)
                        .unwrap()
                        .captures(line)
                        .and_then(|c| c.get(1))
                        .map(|m| m.as_str().to_string())
                })
                .unwrap_or_else(|| "Unknown quality".to_string());
            pending_quality = Some(quality);
        } else if !line.is_empty() && !line.starts_with('#') {
            if let Some(quality) = pending_quality.take() {
                let url = base
                    .join(line)
                    .context("invalid variant playlist URL")?
                    .to_string();
                sources.push(StreamSource { quality, url });
            }
        }
    }
    sources.sort_by_key(|s| {
        std::cmp::Reverse(s.quality.trim_end_matches('p').parse::<u32>().unwrap_or(0))
    });
    if sources.is_empty() {
        bail!("The HLS master playlist contained no quality variants");
    }
    Ok(sources)
}

fn parse_subtitles(config: &Value) -> Vec<SubtitleTrack> {
    config
        .get("subtitles")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let url = s.get("src")?.as_str()?.to_string();
            Some(SubtitleTrack {
                label: s.get("label").and_then(Value::as_str).map(str::to_string),
                language: s
                    .get("lang")
                    .or_else(|| s.get("language"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                is_default: s.get("default").and_then(Value::as_bool).unwrap_or(false),
                url,
            })
        })
        .collect()
}

fn parse_audio_url(master_url: &str, playlist: &str) -> Option<String> {
    let base = Url::parse(master_url).ok()?;
    for line in playlist.lines().map(str::trim) {
        if line.starts_with("#EXT-X-MEDIA:TYPE=AUDIO,") && line.contains("DEFAULT=YES") {
            let uri_re = Regex::new(r#"URI="([^"]+)""#).ok()?;
            let uri = uri_re.captures(line)?.get(1)?.as_str();
            return Some(base.join(uri).ok()?.to_string());
        }
    }
    None
}

/// Choose a quality source. `best`, `worst`, or an exact label such as `720p`.
pub fn select_quality(sources: &[StreamSource], quality: &str) -> Result<StreamSource> {
    if sources.is_empty() {
        bail!("No stream sources are available");
    }
    match quality.to_ascii_lowercase().as_str() {
        "best" => Ok(sources[0].clone()),
        "worst" => Ok(sources.last().unwrap().clone()),
        requested => sources
            .iter()
            .find(|s| s.quality.to_ascii_lowercase().contains(requested))
            .cloned()
            .ok_or_else(|| anyhow!("Quality '{quality}' was not found")),
    }
}
