use once_cell::sync::Lazy;
use semver::Version;
// use sha2::Digest;

use std::{
    ffi::OsString,
    fs,
    io::{Cursor, Write},
    path::PathBuf,
};

use std::time::Duration;
/// Use permissions extensions on unix
#[cfg(target_family = "unix")]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

mod error;
pub use error::VyperVmError;

mod platform;
pub use platform::{platform, Platform};

mod releases;
pub use releases::{all_releases, Releases};

#[cfg(feature = "blocking")]
pub use releases::blocking_all_releases;

/// Declare path to Vyper Version Manager's home directory, "~/.vvm" on Unix-based machines.
pub static VVM_HOME: Lazy<PathBuf> = Lazy::new(|| {
    cfg_if::cfg_if! {
        if #[cfg(test)] {
            let dir = tempfile::tempdir().expect("could not create temp directory");
            dir.path().join(".vvm")
        } else {
            let mut user_home = home::home_dir().expect("could not detect user home directory");
            user_home.push(".vvm");
            user_home
        }
    }
});

/// The timeout to use for requests to the source
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

// Installer type that copies binary data to the appropriate Vyper binary file:
// 1. create target file to copy binary data
// 2. copy data
struct Installer {
    // version of vyper
    version: Version,
    // binary data of the Vyper executable
    binbytes: Vec<u8>,
}

impl Installer {
    /// Installs the vyper version at the version specific destination and returns the path to the installed Vyper file.
    fn install(&self) -> Result<PathBuf, VyperVmError> {
        let version_path = version_path(self.version.to_string().as_str());
        let vyper_path = version_path.join(&format!("vyper-{}", self.version));
        // create vyper file.
        let mut f = fs::File::create(&vyper_path)?;

        #[cfg(target_family = "unix")]
        f.set_permissions(Permissions::from_mode(0o777))?;

        // copy contents over
        let mut content = Cursor::new(&self.binbytes);
        std::io::copy(&mut content, &mut f)?;

        Ok(vyper_path)
    }
}

/// Derive path to a specific Vyper version's binary.
pub fn version_path(version: &str) -> PathBuf {
    let mut version_path = VVM_HOME.to_path_buf();
    version_path.push(&version);
    version_path
}

/// Derive path to VVM's global version file.
pub fn global_version_path() -> PathBuf {
    let mut global_version_path = VVM_HOME.to_path_buf();
    global_version_path.push(".global-version");
    global_version_path
}

/// Reads the currently set global version for Vyper. Returns None if none has yet been set.
pub fn current_version() -> Result<Option<Version>, VyperVmError> {
    let v = fs::read_to_string(global_version_path().as_path())?;
    Ok(Version::parse(v.trim_end_matches('\n').to_string().as_str()).ok())
}

/// Sets the provided version as the global version for Vyper.
pub fn use_version(version: &Version) -> Result<(), VyperVmError> {
    let mut v = fs::File::create(global_version_path().as_path())?;
    v.write_all(version.to_string().as_bytes())?;
    Ok(())
}

/// Unset the global version. This should be done if all versions are removed.
pub fn unset_global_version() -> Result<(), VyperVmError> {
    let mut v = fs::File::create(global_version_path().as_path())?;
    v.write_all("".as_bytes())?;
    Ok(())
}

/// Reads the list of Vyper versions that have been installed in the machine. The version list is
/// sorted in ascending order.
pub fn installed_versions() -> Result<Vec<Version>, VyperVmError> {
    let home_dir = VVM_HOME.to_path_buf();
    println!("home_dir {:?}", &home_dir);
    let mut versions = vec![];
    for v in fs::read_dir(&home_dir)? {
        let v = v?;
        if v.file_name() != OsString::from(".global-version".to_string()) {
            versions.push(Version::parse(
                &v.path()
                    .file_name()
                    .ok_or(VyperVmError::UnknownVersion)?
                    .to_str()
                    .ok_or(VyperVmError::UnknownVersion)?
                    .to_string()
                    .as_str()
                    .replace("vyper-", ""),
            )?);
        }
    }
    versions.sort();

    Ok(versions)
}

/// Blocking version of [`all_versions`]
#[cfg(feature = "blocking")]
pub fn blocking_all_versions() -> Result<Vec<Version>, VyperVmError> {
    Ok(releases::blocking_all_releases(platform::platform())?.into_versions())
}

/// Fetches the list of all the available versions of Vyper. The list is platform dependent, so
/// different versions can be found for macosx vs linux.
pub async fn all_versions() -> Result<Vec<Version>, VyperVmError> {
    Ok(releases::all_releases(platform::platform())
        .await?
        .into_versions())
}

/// Blocking version of [`install`]
#[cfg(feature = "blocking")]
pub fn blocking_install(version: &Version) -> Result<PathBuf, VyperVmError> {
    setup_home()?;

    let artifacts = releases::blocking_all_releases(platform::platform())?;
    let artifact = artifacts
        .get_artifact(version)
        .ok_or(VyperVmError::UnknownVersion)?;
    let download_url =
        releases::artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    // TODO: implement checksum for vyper binaries
    // let checksum = artifacts
    //     .get_checksum(version)
    //     .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    let res = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("reqwest::Client::new()")
        .get(download_url.clone())
        .send()?;

    if !res.status().is_success() {
        return Err(VyperVmError::UnsuccessfulResponse(
            download_url,
            res.status(),
        ));
    }

    let binbytes = res.bytes()?;
    // TODO: implement checksum for vyper binaries
    // ensure_checksum(&binbytes, version, checksum)?;

    // lock file to indicate that installation of this Vyper version will be in progress.
    let lock_path = lock_file_path(version);
    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of Vyper.
    let _lock = try_lock_file(lock_path)?;

    do_install(
        version.clone(),
        binbytes.to_vec(),
        artifact.to_string().as_str(),
    )
}

/// Installs the provided version of Vyper in the machine.
///
/// Returns the path to the Vyper file.
pub async fn install(version: &Version) -> Result<PathBuf, VyperVmError> {
    setup_home()?;

    let artifacts = releases::all_releases(platform::platform()).await?;
    let artifact = artifacts
        .releases
        .get(version)
        .ok_or(VyperVmError::UnknownVersion)?;
    let download_url =
        releases::artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    // TODO: implement checksum for vyper binaries
    // let checksum = artifacts
    //     .get_checksum(version)
    //     .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    let res = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("reqwest::Client::new()")
        .get(download_url.clone())
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(VyperVmError::UnsuccessfulResponse(
            download_url,
            res.status(),
        ));
    }

    let binbytes = res.bytes().await?;
    // TODO: implement checksum for vyper binaries
    // ensure_checksum(&binbytes, version, checksum)?;

    // lock file to indicate that installation of this Vyper version will be in progress.
    let lock_path = lock_file_path(version);
    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of Vyper.
    let _lock = try_lock_file(lock_path)?;

    do_install(
        version.clone(),
        binbytes.to_vec(),
        artifact.to_string().as_str(),
    )
}

fn do_install(
    version: Version,
    binbytes: Vec<u8>,
    _artifact: &str,
) -> Result<PathBuf, VyperVmError> {
    let installer = {
        setup_version(version.to_string().as_str())?;

        Installer { version, binbytes }
    };

    installer.install()
}

/// Removes the provided version of Vyper from the machine.
pub fn remove_version(version: &Version) -> Result<(), VyperVmError> {
    fs::remove_dir_all(version_path(version.to_string().as_str()))?;
    Ok(())
}

/// Setup SVM home directory.
pub fn setup_home() -> Result<PathBuf, VyperVmError> {
    // create ~/.svm
    let home_dir = VVM_HOME.to_path_buf();
    if !home_dir.as_path().exists() {
        fs::create_dir_all(home_dir.clone())?;
    }
    // create ~/.svm/.global-version
    let mut global_version = VVM_HOME.to_path_buf();
    global_version.push(".global-version");
    if !global_version.as_path().exists() {
        fs::File::create(global_version.as_path())?;
    }
    Ok(home_dir)
}

fn setup_version(version: &str) -> Result<(), VyperVmError> {
    let v = version_path(version);
    if !v.exists() {
        fs::create_dir_all(v.as_path())?
    }
    Ok(())
}

// TODO: implement checksum for vyper binaries
// fn ensure_checksum(
//     binbytes: impl AsRef<[u8]>,
//     version: &Version,
//     expected_checksum: Vec<u8>,
// ) -> Result<(), VyperVmError> {
//     let mut hasher = sha2::Sha256::new();
//     hasher.update(binbytes);
//     let cs = &hasher.finalize()[..];
//     // checksum does not match
//     if cs != expected_checksum {
//         return Err(VyperVmError::ChecksumMismatch(version.to_string()));
//     }
//     Ok(())
// }

/// Creates the file and locks it exclusively, this will block if the file is currently locked
fn try_lock_file(lock_path: PathBuf) -> Result<LockFile, VyperVmError> {
    use fs2::FileExt;
    let _lock_file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    _lock_file.lock_exclusive()?;
    Ok(LockFile {
        lock_path,
        _lock_file,
    })
}

/// Represents a lockfile that's removed once dropped
struct LockFile {
    _lock_file: fs::File,
    lock_path: PathBuf,
}

impl Drop for LockFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Returns the lockfile to use for a specific file
fn lock_file_path(version: &Version) -> PathBuf {
    VVM_HOME.join(&format!(".lock-vyper-{}", version))
}

#[cfg(test)]
mod tests {
    use crate::{
        platform::Platform,
        releases::{all_releases, artifact_url},
    };
    use rand::seq::SliceRandom;
    use reqwest::Url;

    use std::process::{Command, Stdio};

    use super::*;

    #[tokio::test]
    async fn test_artifact_url() {
        let version = Version::new(0, 3, 3);
        let version_str = version.to_string();
        let artifact = &format!("vyper.{}%2Bcommit.48e326f0.darwin", version_str);
        assert_eq!(
            artifact_url(Platform::Linux, &version, artifact).unwrap(),
            Url::parse(&format!(
                "https://github.com/vyperlang/vyper/releases/download/v{}/{}",
                version_str, artifact
            ))
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn test_install() {
        let versions = all_releases(platform())
            .await
            .unwrap()
            .releases
            .into_keys()
            .collect::<Vec<Version>>();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(install(rand_version).await.is_ok());
    }

    #[tokio::test]
    async fn test_installed_versions() {
        let versions = all_releases(platform())
            .await
            .unwrap()
            .releases
            .into_keys()
            .collect::<Vec<Version>>();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(install(rand_version).await.is_ok());
        let installed_versions = installed_versions().unwrap_or_default();
        assert!(&installed_versions.len() > &0);
        assert!(&installed_versions.contains(rand_version));
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn blocking_test_install() {
        let versions = crate::releases::blocking_all_releases(platform::platform())
            .unwrap()
            .into_versions();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(blocking_install(rand_version).is_ok());
    }

    #[tokio::test]
    async fn test_version() {
        let version = "0.3.3".parse().unwrap();
        install(&version).await.unwrap();
        let vyper_path =
            version_path(version.to_string().as_str()).join(&format!("vyper-{}", version));
        let output = Command::new(&vyper_path)
            .arg("--version")
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("0.3.3"));
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn blocking_test_version() {
        let version = "0.3.3".parse().unwrap();
        blocking_install(&version).unwrap();
        let vyper_path =
            version_path(version.to_string().as_str()).join(&format!("vyper-{}", version));
        let output = Command::new(&vyper_path)
            .arg("--version")
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("0.3.3"));
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn can_install_parallel() {
        let version: Version = "0.3.3".parse().unwrap();
        let cloned_version = version.clone();
        let t = std::thread::spawn(move || blocking_install(&cloned_version));
        blocking_install(&version).unwrap();
        t.join().unwrap().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_install_parallel_async() {
        let version: Version = "0.3.3".parse().unwrap();
        let cloned_version = version.clone();
        let t = tokio::task::spawn(async move { install(&cloned_version).await });
        install(&version).await.unwrap();
        t.await.unwrap().unwrap();
    }

    // ensures we can download the latest native Vyper for apple silicon
    #[tokio::test(flavor = "multi_thread")]
    async fn can_download_latest_macos() {
        let latest: Version = "0.3.3".parse().unwrap();

        let artifacts = all_releases(Platform::MacOs).await.unwrap();

        let artifact = artifacts.releases.get(&latest).unwrap();
        let download_url =
            artifact_url(Platform::MacOs, &latest, artifact.to_string().as_str()).unwrap();
        // TODO: implement checksum for vyper binaries
        // let checksum = artifacts.get_checksum(&latest).unwrap();

        let resp = reqwest::get(download_url).await.unwrap();
        assert!(resp.status().is_success());
        let _binbytes = resp.bytes().await.unwrap();
        // TODO: implement checksum for vyper binaries
        // ensure_checksum(&binbytes, &latest, checksum).unwrap();
    }
}
