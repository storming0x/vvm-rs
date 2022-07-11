use std::{env, process::Command};

fn main() -> anyhow::Result<()> {
    let args = env::args().skip(1).collect::<Vec<String>>();

    let version = vvm_lib::current_version()?.ok_or(vvm_lib::VyperVmError::GlobalVersionNotSet)?;
    let mut version_path = vvm_lib::version_path(version.to_string().as_str());
    version_path.push(format!("vyper-{}", version.to_string().as_str()));

    Command::new(version_path).args(args).spawn()?;

    Ok(())
}
