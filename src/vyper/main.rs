mod cache;
mod error;

use cache::VyperFilesCache;
use std::{
    env, fs,
    process::{Command, Stdio},
};

use crate::error::VyperError;

#[tokio::main]
async fn main() -> error::Result<()> {
    let args = env::args().skip(1).collect::<Vec<String>>();

    // setup .vvm/ dir in home directory
    vvm_lib::setup_home()?;

    let mut cache = VyperFilesCache::get();
    let file_name = fs::canonicalize(&args[0]).map_err(|err| VyperError::io(err, &args[0]))?;

    if args.len() == 1 && !args[0].starts_with('-') {
        // support cache only for single file inputs
        if let Some(entry) = cache.entry(file_name.clone()) {
            if !entry.is_dirty() {
                // print out cached version
                println!("{}", entry.deployed_bytecode);
                return Ok(());
            }
        }
    }

    // if we are here it means cache entry was not found or was dirty
    // compile as normal and update/create cache file
    let version = vvm_lib::current_version()?.ok_or(vvm_lib::VyperVmError::GlobalVersionNotSet)?;
    let mut version_path = vvm_lib::version_path(version.to_string().as_str());
    version_path.push(format!("vyper-{}", version.to_string().as_str()));

    let child = Command::new(version_path)
        .args(args.clone())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Vyper wrapper: failed to execute vyper command");

    let output = child
        .wait_with_output()
        .expect("Vyper wrapper: failed to wait for child output");

    if output.status.success() {
        println!("{}", std::str::from_utf8(&output.stdout).unwrap());
        // cache house keeping
        if args.len() == 1 && !args[0].starts_with('-') {
            if let Some(bytecode) = get_bytecode(&output.stdout) {
                if cache.add_entry(file_name, &bytecode).is_ok() {
                    let _ = cache.write(cache::get_cache_path());
                    // ignore errors
                    // TODO: add debug statements
                }
            }
        }
    } else {
        println!("{}", std::str::from_utf8(&output.stderr).unwrap());
    }

    Ok(())
}

fn get_bytecode(bytecode: &[u8]) -> Option<String> {
    return match std::str::from_utf8(bytecode) {
        Ok(b) if b.starts_with("0x") => Some(b.to_string()),
        Ok(_) => None,
        _ => None,
    };
}
