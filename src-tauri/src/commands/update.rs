use std::time::Duration;

use reqwest::{
    header::{ACCEPT, USER_AGENT},
    Client, Proxy, Url,
};
use serde::{Deserialize, Serialize};
use tauri::{utils::config::BundleType, utils::platform::bundle_type, AppHandle};
use tauri_plugin_updater::UpdaterExt;

const GITHUB_RELEASES_API_URL: &str =
    "https://api.github.com/repos/88lin/lessAI/releases?per_page=50";
const RELEASE_MANIFEST_URL_TEMPLATE: &str =
    "https://github.com/88lin/lessAI/releases/download/{tag}/latest.json";
const RELEASES_USER_AGENT: &str = "LessAI-VersionManager/1.0";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseVersionSummary {
    pub tag: String,
    pub version: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub html_url: String,
    pub published_at: Option<String>,
    pub prerelease: bool,
    pub updater_available: bool,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    published_at: Option<String>,
    draft: bool,
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
}

fn normalize_proxy_url(raw_proxy: Option<String>) -> Result<Option<String>, String> {
    let Some(proxy) = raw_proxy else {
        return Ok(None);
    };

    let proxy = proxy.trim();
    if proxy.is_empty() {
        return Ok(None);
    }

    let normalized = if proxy.contains("://") {
        proxy.to_string()
    } else {
        format!("http://{proxy}")
    };

    Url::parse(&normalized).map_err(|error| format!("代理地址无效：{error}"))?;
    Ok(Some(normalized))
}

fn normalize_release_tag(tag: &str) -> Result<String, String> {
    let tag = tag.trim();
    if tag.is_empty() {
        return Err("版本号不能为空。".to_string());
    }

    Ok(if tag.starts_with('v') {
        tag.to_string()
    } else {
        format!("v{tag}")
    })
}

fn normalize_version_from_tag(tag: &str) -> String {
    tag.trim_start_matches('v').to_string()
}

fn build_reqwest_client(proxy: Option<String>, timeout_secs: u64) -> Result<Client, String> {
    let mut builder = Client::builder().timeout(Duration::from_secs(timeout_secs));
    if let Some(proxy) = normalize_proxy_url(proxy)? {
        let reqwest_proxy = Proxy::all(proxy).map_err(|error| format!("代理配置失败：{error}"))?;
        builder = builder.proxy(reqwest_proxy);
    }
    builder
        .build()
        .map_err(|error| format!("网络客户端初始化失败：{error}"))
}

#[tauri::command]
pub async fn list_release_versions(
    proxy: Option<String>,
) -> Result<Vec<ReleaseVersionSummary>, String> {
    let client = build_reqwest_client(proxy, 15)?;
    let response = client
        .get(GITHUB_RELEASES_API_URL)
        .header(USER_AGENT, RELEASES_USER_AGENT)
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| format!("拉取版本列表失败：{error}"))?;

    if !response.status().is_success() {
        return Err(format!("拉取版本列表失败：HTTP {}", response.status()));
    }

    let releases: Vec<GithubRelease> = response
        .json()
        .await
        .map_err(|error| format!("解析版本列表失败：{error}"))?;

    let mut result = Vec::with_capacity(releases.len());
    for release in releases.into_iter().filter(|item| !item.draft) {
        let tag = normalize_release_tag(&release.tag_name)?;
        let updater_available = release
            .assets
            .iter()
            .any(|asset| asset.name.eq_ignore_ascii_case("latest.json"));
        result.push(ReleaseVersionSummary {
            version: normalize_version_from_tag(&tag),
            tag,
            name: release.name,
            body: release.body,
            html_url: release.html_url,
            published_at: release.published_at,
            prerelease: release.prerelease,
            updater_available,
        });
    }

    Ok(result)
}

#[tauri::command]
pub async fn switch_release_version(
    app: AppHandle,
    tag: String,
    proxy: Option<String>,
) -> Result<String, String> {
    if matches!(bundle_type(), Some(BundleType::Deb) | Some(BundleType::Rpm)) {
        return Err("当前安装包类型不支持应用内切换版本，请手动下载安装新版本。".to_string());
    }

    let tag = normalize_release_tag(&tag)?;
    let endpoint = RELEASE_MANIFEST_URL_TEMPLATE.replace("{tag}", &tag);
    let endpoint = Url::parse(&endpoint).map_err(|error| format!("构建更新地址失败：{error}"))?;

    let mut builder = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|error| format!("配置版本更新源失败：{error}"))?
        .version_comparator(|current, remote| current != remote.version)
        .timeout(Duration::from_secs(20));

    if let Some(proxy) = normalize_proxy_url(proxy)? {
        let proxy = Url::parse(&proxy).map_err(|error| format!("代理地址无效：{error}"))?;
        builder = builder.proxy(proxy);
    }

    let updater = builder
        .build()
        .map_err(|error| format!("初始化更新器失败：{error}"))?;

    let Some(update) = updater
        .check()
        .await
        .map_err(|error| format!("检查目标版本失败：{error}"))?
    else {
        return Err(format!(
            "未发现可安装版本：{tag}。请确认该发布包含 latest.json 与当前平台更新包。"
        ));
    };

    let installed_version = update.version.to_string();
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| format!("安装版本 {tag} 失败：{error}"))?;

    Ok(installed_version)
}
