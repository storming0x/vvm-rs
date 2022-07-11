use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use semver::Version;
use serde::{
    de::{self, Deserializer},
    Deserialize, Serialize,
};
use std::collections::BTreeMap;
use url::Url;

use crate::{error::VyperVmError, platform::Platform};

const GITHUB_RELEASES: &str = "https://api.github.com/repos/vyperlang/vyper/releases?per_page=100";

/// Defines the struct that the JSON-formatted release list can be deserialized into.
///
/// {
///     "tag_name": "v0.3.3",
///     ...
///     "assets": [
///       {
///         "name": "vyper.0.3.3+commit.48e326f0.darwin",
///         ...
///         "browser_download_url": "https://github.com/vyperlang/vyper/releases/download/v0.3.3/vyper.0.3.3%2Bcommit.48e326f0.darwin"
///        }
///     ]
/// }
///
///
///
///
#[derive(Debug, Serialize, Deserialize)]
struct VyperAsset {
    name: String,
    browser_download_url: String,
}
/// Both the key and value are deserialized into semver::Version.
#[derive(Debug, Serialize, Deserialize)]
struct VyperReleases {
    tag_name: String,
    assets: Vec<VyperAsset>,
}

/// Both the key and value are deserialized into semver::Version.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Releases {
    pub builds: Vec<BuildInfo>,
    pub releases: BTreeMap<Version, String>,
}

impl Releases {
    /// NOTE: vyper binaries dont support checksums
    pub fn get_checksum(&self, v: &Version) -> Option<Vec<u8>> {
        for build in self.builds.iter() {
            if build.version.eq(v) {
                return Some(build.sha256.clone());
            }
        }
        None
    }

    /// Returns the artifact of the version if any
    pub fn get_artifact(&self, version: &Version) -> Option<&String> {
        self.releases.get(version)
    }

    /// Returns a sorted list of all versions
    pub fn into_versions(self) -> Vec<Version> {
        let mut versions = self.releases.into_keys().collect::<Vec<_>>();
        versions.sort_unstable();
        versions
    }
}

/// Build info contains the SHA256 checksum of a solc binary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    pub version: Version,
    #[serde(with = "hex_string")]
    pub sha256: Vec<u8>,
}

/// Helper serde module to serialize and deserialize bytes as hex.
mod hex_string {
    use super::*;
    use serde::Serializer;
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str_hex = String::deserialize(deserializer)?;
        let str_hex = str_hex.trim_start_matches("0x");
        hex::decode(str_hex).map_err(|err| de::Error::custom(err.to_string()))
    }

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: AsRef<[u8]>,
    {
        let value = hex::encode(value);
        serializer.serialize_str(&value)
    }
}

/// Blocking version for [`all_releases`]
#[cfg(feature = "blocking")]
pub fn blocking_all_releases(platform: Platform) -> Result<Releases, VyperVmError> {
    let vyper_releases = blocking_get_releases()?;

    let mut builds: Vec<BuildInfo> = Vec::new();
    let mut releases: BTreeMap<Version, String> = BTreeMap::new();
    let platform_str = &platform.to_string();
    for vyper_release in vyper_releases {
        for asset in vyper_release.assets {
            if asset.name.contains(platform_str) {
                let version =
                    Version::parse(&vyper_release.tag_name.trim_start_matches("v")).unwrap();
                builds.push(BuildInfo {
                    version: version.clone(),
                    sha256: Vec::new(),
                });
                releases.insert(version, asset.name);
            }
        }
    }

    Ok(Releases { builds, releases })
}

/// Fetch all releases available for the provided platform.
pub async fn all_releases(platform: Platform) -> Result<Releases, VyperVmError> {
    let vyper_releases = get_releases().await?;

    let mut builds: Vec<BuildInfo> = Vec::new();
    let mut releases: BTreeMap<Version, String> = BTreeMap::new();
    let platform_str = &platform.to_string();
    for vyper_release in vyper_releases {
        for asset in vyper_release.assets {
            if asset.name.contains(platform_str) {
                let version =
                    Version::parse(vyper_release.tag_name.trim_start_matches('v')).unwrap();
                builds.push(BuildInfo {
                    version: version.clone(),
                    sha256: Vec::new(),
                });
                releases.insert(version, asset.name);
            }
        }
    }

    Ok(Releases { builds, releases })
}

async fn get_releases() -> Result<Vec<VyperReleases>, VyperVmError> {
    let mut headers = HeaderMap::new();
    // add the user-agent header required by github
    headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));

    let vyper_releases = reqwest::Client::new()
        .get(GITHUB_RELEASES)
        .headers(headers)
        .send()
        .await?
        .json::<Vec<VyperReleases>>()
        .await?;

    Ok(vyper_releases)
}

#[allow(dead_code)]
fn blocking_get_releases() -> Result<Vec<VyperReleases>, VyperVmError> {
    let mut headers = HeaderMap::new();
    // add the user-agent header required by github
    headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
    let vyper_releases = reqwest::blocking::Client::new()
        .get(GITHUB_RELEASES)
        .headers(headers)
        .send()?
        .json::<Vec<VyperReleases>>()?;

    Ok(vyper_releases)
}

/// Construct the URL to the Vyper binary for the specified release version and target platform.
pub fn artifact_url(
    _platform: Platform,
    version: &Version,
    artifact: &str,
) -> Result<Url, VyperVmError> {
    Ok(Url::parse(&format!(
        "https://github.com/vyperlang/vyper/releases/download/v{}/{}",
        &version.to_string(),
        artifact
    ))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[tokio::test]
    // async fn test_macos_aarch64() {
    //     let releases = all_releases(Platform::MacOs)
    //         .await
    //         .expect("could not fetch releases for macos-aarch64");
    //     let rosetta = Version::new(0, 8, 4);
    //     let native = MACOS_AARCH64_NATIVE.clone();
    //     let url1 = artifact_url(
    //         Platform::MacOsAarch64,
    //         &rosetta,
    //         releases.get_artifact(&rosetta).unwrap(),
    //     )
    //     .expect("could not fetch artifact URL");
    //     let url2 = artifact_url(
    //         Platform::MacOsAarch64,
    //         &native,
    //         releases.get_artifact(&native).unwrap(),
    //     )
    //     .expect("could not fetch artifact URL");
    //     assert!(url1.to_string().contains(SOLC_RELEASES_URL));
    //     assert!(url2.to_string().contains(MACOS_AARCH64_URL_PREFIX));
    // }

    #[tokio::test]
    async fn test_all_releases_macos() {
        assert!(all_releases(Platform::MacOs).await.is_ok());
    }

    #[tokio::test]
    async fn test_all_releases_linux() {
        assert!(all_releases(Platform::Linux).await.is_ok());
    }

    #[tokio::test]
    async fn releases_roundtrip() {
        let releases = all_releases(Platform::Linux).await.unwrap();
        let s = serde_json::to_string(&releases).unwrap();
        let de_releases: Releases = serde_json::from_str(&s).unwrap();
        assert_eq!(releases, de_releases);
    }
}
