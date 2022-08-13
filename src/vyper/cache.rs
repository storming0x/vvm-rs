use crate::error::{Result, VyperError};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::btree_map::BTreeMap,
    fs::{self},
    io,
    path::{Path, PathBuf},
};
use vvm_lib::VVM_HOME;

use md5::Digest;

// simplified cache based on ether-rs solc logic with adjustments for vyper
// https://github.com/gakonst/ethers-rs/blob/c75608eda1e1fdc7366a7501c1a6b3f0216a25ea/ethers-solc/src/cache.rs

// close to ether-rs solidity cache format
const FORMAT_VERSION: &str = "vvm-rs-vyper-cache-1";

/// The file name of the default cache file
pub const VYPER_FILES_CACHE_FILENAME: &str = "vvm-vyper-files-cache.json";

/// A cache file
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VyperFilesCache {
    #[serde(rename = "_format")]
    pub format: String,
    pub files: BTreeMap<PathBuf, CacheEntry>,
}

impl VyperFilesCache {
    /// Create a new cache instance with empty entries
    fn new() -> Self {
        Self {
            format: FORMAT_VERSION.to_string(),
            files: BTreeMap::new(),
        }
    }

    // loads existing cache or create a new one
    pub fn get() -> Self {
        if let Ok(cache) = VyperFilesCache::read(get_cache_path()) {
            cache
        } else {
            return VyperFilesCache::new();
        }
    }

    // pub fn is_empty(&self) -> bool {
    //     self.files.is_empty()
    // }

    /// How many entries the cache contains where each entry represents a source file
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns the corresponding `CacheEntry` for the file if it exists
    pub fn entry(&self, file: impl AsRef<Path>) -> Option<&CacheEntry> {
        self.files.get(file.as_ref())
    }

    /// Returns the corresponding `CacheEntry` for the file if it exists
    pub fn entry_mut(&mut self, file: impl AsRef<Path>) -> Option<&mut CacheEntry> {
        self.files.get_mut(file.as_ref())
    }

    /// adds or updates an entry in cache
    pub fn add_entry(&mut self, file: impl AsRef<Path>, bytecode: &str) -> Result<()> {
        if let Some(mut entry) = self.entry_mut(file.as_ref()) {
            // update
            entry.content_hash = get_file_hash(file.as_ref())?;
            entry.deployed_bytecode = bytecode.to_string();
        }

        // add new entry
        let new_entry = CacheEntry {
            content_hash: get_file_hash(file.as_ref())?,
            source_name: file.as_ref().to_path_buf(),
            deployed_bytecode: bytecode.to_string(),
        };

        self.files.insert(file.as_ref().to_path_buf(), new_entry);

        Ok(())
    }

    /// Reads the cache json file from the given path
    #[tracing::instrument(skip_all, name = "vyper-files-cache::read")]
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        tracing::trace!("reading vyper files cache at {}", path.display());
        let cache: VyperFilesCache = read_json_file(path)?;
        tracing::trace!(
            "read cache \"{}\" with {} entries",
            cache.format,
            cache.files.len()
        );
        Ok(cache)
    }

    /// Write the cache as json file to the given path
    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        create_parent_dir_all(path)?;
        let file = fs::File::create(path).map_err(|err| VyperError::io(err, path))?;
        tracing::trace!(
            "writing cache with {} entries to json file: \"{}\"",
            self.len(),
            path.display()
        );
        serde_json::to_writer_pretty(file, self)?;
        tracing::trace!("cache file located: \"{}\"", path.display());
        Ok(())
    }
}

/// A `CacheEntry` in the cache file represents a vyper file
///
/// TODO: add better cache invalidation of entries
/// Simplified version of cache entry for basic caching
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntry {
    /// hash to identify whether the content of the file changed
    pub content_hash: String,
    /// identifier name
    pub source_name: PathBuf,
    // TODO: implement version
    // pub version_requirement: Option<String>,
    // TODO: implement version
    // pub last_modified: : u6,
    pub deployed_bytecode: String,
}

impl CacheEntry {
    ///  returns true file:
    ///   - is new
    ///   - has changed
    ///  returns false if file si found and hash is the same
    pub fn is_dirty(&self) -> bool {
        if let Ok(hash) = get_file_hash(&self.source_name) {
            if hash == self.content_hash {
                return false;
            }
        }

        return true;
    }
}

///// Helper Functions /////

fn get_file_hash(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let file = std::fs::File::open(path).map_err(|err| VyperError::io(err, path))?;
    let mut file = std::io::BufReader::new(file);

    let mut hasher = md5::Md5::new();
    let _ = io::copy(&mut file, &mut hasher).map_err(|err| VyperError::io(err, path))?;
    let result = hasher.finalize();

    Ok(hex::encode(result))
}

/// Reads the json file and deserialize it into the provided type
fn read_json_file<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let file = std::fs::File::open(path).map_err(|err| VyperError::io(err, path))?;
    let file = std::io::BufReader::new(file);
    let val: T = serde_json::from_reader(file)?;
    Ok(val)
}

/// Creates the parent directory of the `file` and all its ancestors if it does not exist
/// See [`std::fs::create_dir_all()`]
fn create_parent_dir_all(file: impl AsRef<Path>) -> Result<()> {
    let file = file.as_ref();
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            VyperError::msg(format!(
                "Failed to create artifact parent folder \"{}\": {}",
                parent.display(),
                err
            ))
        })?;
    }
    Ok(())
}

/// Get cache dir path
pub fn get_cache_path() -> PathBuf {
    let mut cache_path = VVM_HOME.to_path_buf();
    cache_path.push("cache");
    cache_path.push(VYPER_FILES_CACHE_FILENAME);
    cache_path
}

#[test]
fn test_read_cache_file() -> Result<()> {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push(format!("test-data/{}", VYPER_FILES_CACHE_FILENAME));

    let file_name = "test-data/Token.vy";

    println!("path read cache {}", d.display());

    let cache_result = VyperFilesCache::read(d);

    assert!(cache_result.is_ok());

    let cache = cache_result.unwrap();

    assert_eq!(cache.len(), 1);

    let cache_entry = cache
        .entry(file_name)
        .expect("expected cache entry to exist");

    let CacheEntry {
        source_name,
        content_hash,
        deployed_bytecode,
    } = cache_entry;

    assert_eq!(source_name.as_os_str(), file_name);
    assert_eq!(content_hash, "b95e2a6f5312b7df45db0caa631f2d21");
    assert_eq!(
        deployed_bytecode,
        r#"0x61048561001161000039610485610000f36003361161000c5761046d565b60003560e01c34610473576306fdde03811861009f576004361861047357602080608052600a6040527f5465737420546f6b656e0000000000000000000000000000000000000000000060605260408160800181518082526020830160208301815181525050508051806020830101601f82600003163682375050601f19601f8251602001011690509050810190506080f35b6395d89b41811861012757600436186104735760208060805260046040527f544553540000000000000000000000000000000000000000000000000000000060605260408160800181518082526020830160208301815181525050508051806020830101601f82600003163682375050601f19601f8251602001011690509050810190506080f35b63313ce5678118610145576004361861047357601260405260206040f35b63a9059cbb81186101eb5760443618610473576004358060a01c610473576040526001336020526000526040600020805460243580820382811161047357905090508155506001604051602052600052604060002080546024358082018281106104735790509050815550604051337fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef60243560605260206060a3600160605260206060f35b63095ea7b3811861026a5760443618610473576004358060a01c610473576040526024356002336020526000526040600020806040516020526000526040600020905055604051337f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b92560243560605260206060a3600160605260206060f35b6323b872dd81186103575760643618610473576004358060a01c610473576040526024358060a01c610473576060526002604051602052600052604060002080336020526000526040600020905080546044358082038281116104735790509050815550600160405160205260005260406000208054604435808203828111610473579050905081555060016060516020526000526040600020805460443580820182811061047357905090508155506060516040517fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef60443560805260206080a3600160805260206080f35b6341a9680381186103b75760443618610473576004358060a01c6104735760405260016040516020526000526040600020805460243580820182811061047357905090508155506000546024358082018281106104735790509050600055005b6318160ddd81186103d657600436186104735760005460405260206040f35b6370a0823181186104115760243618610473576004358060a01c61047357604052600160405160205260005260406000205460605260206060f35b63dd62ed3e811861046b5760443618610473576004358060a01c610473576040526024358060a01c610473576060526002604051602052600052604060002080606051602052600052604060002090505460805260206080f35b505b60006000fd5b600080fda165767970657283000306000b"#
    );

    Ok(())
}

#[test]
fn test_get_file_hash() -> Result<()> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test-data/Token.vy");

    let expected_hash = "089f6055c2d023b76eed71e820e7b580";
    let hash = get_file_hash(path)?;
    assert_eq!(hash, expected_hash);

    Ok(())
}

#[test]
fn test_cache_entry_is_dirty() -> Result<()> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test-data/Token.vy");

    const BAD_HASH: &str = "b95e2a6f5312b7df45db0caa631f2d21";

    let clean_entry = CacheEntry {
        content_hash: "089f6055c2d023b76eed71e820e7b580".to_string(),
        source_name: path.clone(),
        deployed_bytecode: "mockbytecode".to_string(),
    };

    let dirty_entry = CacheEntry {
        content_hash: BAD_HASH.to_string(),
        source_name: path.clone(),
        deployed_bytecode: "mockbytecode".to_string(),
    };

    assert!(clean_entry.is_dirty() != true);
    assert!(dirty_entry.is_dirty());

    Ok(())
}

#[test]
fn test_add_cache_entry() -> Result<()> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test-data/Token.vy");

    const UPDATED_BYTECODE: &str = "mocknewbytecode";
    const MOCK_BYTECODE: &str = "mockbytecode";
    const CONTENT_HASH: &str = "089f6055c2d023b76eed71e820e7b580";

    let new_entry = CacheEntry {
        content_hash: "089f6055c2d023b76eed71e820e7b580".to_string(),
        source_name: path.clone(),
        deployed_bytecode: MOCK_BYTECODE.to_string(),
    };

    let mut cache = VyperFilesCache::new();

    cache.add_entry(&path, &MOCK_BYTECODE)?;

    assert!(cache.len() > 0);
    let first_entry_op = cache.entry(new_entry.source_name);
    assert!(first_entry_op.is_some());
    let first_entry = first_entry_op.unwrap();
    assert_eq!(first_entry.deployed_bytecode, MOCK_BYTECODE);
    assert_eq!(first_entry.content_hash, CONTENT_HASH);

    // update
    cache.add_entry(&path, &UPDATED_BYTECODE)?;
    assert!(cache.len() == 1);
    let updated_entry = cache.entry(path.clone());
    assert!(updated_entry.is_some());
    assert_eq!(updated_entry.unwrap().deployed_bytecode, UPDATED_BYTECODE);

    Ok(())
}
