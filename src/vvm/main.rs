use clap::Parser;
use dialoguer::Input;
use semver::Version;

use std::collections::HashSet;

mod print;

#[derive(Debug, Parser)]
#[clap(name = "vvm", about = "Vyper Version Manager", version)]
enum VyperVm {
    #[clap(about = "List all versions of Vyper")]
    List,
    #[clap(about = "Install Vyper versions")]
    Install { versions: Vec<String> },
    #[clap(about = "Use a Vyper version")]
    Use { version: String },
    #[clap(about = "Remove a Vyper version")]
    Remove { version: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = VyperVm::parse();

    vvm_lib::setup_home()?;

    match opt {
        VyperVm::List => {
            handle_list().await?;
        }
        VyperVm::Install { versions } => {
            for v in versions {
                handle_install(Version::parse(&v)?).await?;
            }
        }
        VyperVm::Use { version } => {
            handle_use(Version::parse(&version)?).await?;
        }
        VyperVm::Remove { version } => match version.as_str() {
            "ALL" | "all" => {
                for v in vvm_lib::installed_versions().unwrap_or_default() {
                    vvm_lib::remove_version(&v)?;
                }
                vvm_lib::unset_global_version()?;
            }
            _ => handle_remove(Version::parse(&version)?)?,
        },
    }

    Ok(())
}

async fn handle_list() -> anyhow::Result<()> {
    let all_versions = vvm_lib::all_versions().await?;
    let installed_versions = vvm_lib::installed_versions().unwrap_or_default();
    let current_version = vvm_lib::current_version()?;

    let a: HashSet<Version> = all_versions.iter().cloned().collect();
    let b: HashSet<Version> = installed_versions.iter().cloned().collect();
    let c = &a - &b;

    let mut available_versions = c.iter().cloned().collect::<Vec<Version>>();
    available_versions.sort();

    print::current_version(current_version);
    print::installed_versions(installed_versions);
    print::available_versions(available_versions);

    Ok(())
}

async fn handle_install(version: Version) -> anyhow::Result<()> {
    let all_versions = vvm_lib::all_versions().await?;
    let installed_versions = vvm_lib::installed_versions().unwrap_or_default();
    let current_version = vvm_lib::current_version()?;

    if installed_versions.contains(&version) {
        println!("Vyper {} is already installed", version);
        let input: String = Input::new()
            .with_prompt("Would you like to set it as the global version?")
            .with_initial_text("Y")
            .default("N".into())
            .interact_text()?;
        if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
            vvm_lib::use_version(&version)?;
            print::set_global_version(&version);
        }
    } else if all_versions.contains(&version) {
        let spinner = print::installing_version(&version);
        vvm_lib::install(&version).await?;
        spinner.finish_with_message(format!("Downloaded Vyper: {}", version));
        if current_version.is_none() {
            vvm_lib::use_version(&version)?;
            print::set_global_version(&version);
        }
    } else {
        print::unsupported_version(&version);
    }

    Ok(())
}

async fn handle_use(version: Version) -> anyhow::Result<()> {
    let all_versions = vvm_lib::all_versions().await?;
    let installed_versions = vvm_lib::installed_versions().unwrap_or_default();

    if installed_versions.contains(&version) {
        vvm_lib::use_version(&version)?;
        print::set_global_version(&version);
    } else if all_versions.contains(&version) {
        println!("Vyper {} is not installed", version);
        let input: String = Input::new()
            .with_prompt("Would you like to install it?")
            .with_initial_text("Y")
            .default("N".into())
            .interact_text()?;
        if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
            handle_install(version).await?;
        }
    } else {
        print::unsupported_version(&version);
    }

    Ok(())
}

fn handle_remove(version: Version) -> anyhow::Result<()> {
    let mut installed_versions = vvm_lib::installed_versions().unwrap_or_default();
    let current_version = vvm_lib::current_version()?;

    if installed_versions.contains(&version) {
        let input: String = Input::new()
            .with_prompt("Are you sure?")
            .with_initial_text("Y")
            .default("N".into())
            .interact_text()?;
        if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
            vvm_lib::remove_version(&version)?;
            if let Some(v) = current_version {
                if version == v {
                    if let Some(i) = installed_versions.iter().position(|x| *x == v) {
                        installed_versions.remove(i);
                        if let Some(new_version) = installed_versions.pop() {
                            vvm_lib::use_version(&new_version)?;
                            print::set_global_version(&new_version);
                        } else {
                            vvm_lib::unset_global_version()?;
                        }
                    }
                }
            }
        }
    } else {
        print::version_not_found(&version);
    }

    Ok(())
}
