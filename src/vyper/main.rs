use std::{env, process::Command};

fn main() -> anyhow::Result<()> {
    let args = env::args().skip(1).collect::<Vec<String>>();

    let version = vvm_lib::current_version()?.ok_or(vvm_lib::VyperVmError::GlobalVersionNotSet)?;
    let mut version_path = vvm_lib::version_path(version.to_string().as_str());
    version_path.push(format!("vyper-{}", version.to_string().as_str()));

    let child = Command::new(version_path)
        .args(args)
        .spawn()
        .expect("Vyper wrapper: failed to execute vyper command");

    let output = child
        .wait_with_output()
        .expect("Vyper wrapper: failed to wait for child output");

    if output.status.success() {
        println!("{}", std::str::from_utf8(&output.stdout).unwrap());
    } else {
        println!("{}", std::str::from_utf8(&output.stderr).unwrap());
    }

    Ok(())
}
