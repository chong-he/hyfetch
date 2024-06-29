use std::borrow::Cow;
use std::ffi::OsStr;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fmt};

use anyhow::{anyhow, Context, Result};
use tracing::debug;

use crate::distros::Distro;

/// Gets the absolute path of the neofetch command.
pub fn get_command_path() -> Result<PathBuf> {
    if let Ok(workspace_dir) = env::var("CARGO_WORKSPACE_DIR") {
        let path = Path::new(&workspace_dir);
        if path.exists() {
            let path = path.join("neofetch");
            match path.try_exists() {
                Ok(true) => {
                    return path.canonicalize().context("Failed to canonicalize path");
                },
                Ok(false) => {
                    Err(anyhow!("{path:?} does not exist or is not readable"))?;
                },
                Err(err) => {
                    Err(err)
                        .with_context(|| format!("Failed to check for existence of {path:?}"))?;
                },
            }
        }
    }

    let Ok(path_env) = env::var("PATH") else {
        return Err(anyhow!("`PATH` env var is not set or invalid"));
    };

    for search_path in env::split_paths(&path_env) {
        let path = search_path.join("neowofetch");
        if !path.is_file() {
            continue;
        }
        return path.canonicalize().context("Failed to canonicalize path");
    }

    Err(anyhow!("neofetch command not found"))
}

/// Gets the distro ascii of the current distro. Or if distro is specified, get
/// the specific distro's ascii art instead.
#[tracing::instrument(level = "debug")]
pub fn get_distro_ascii<S>(distro: Option<S>) -> Result<String>
where
    S: AsRef<str> + fmt::Debug,
{
    let distro: Cow<_> = if let Some(distro) = distro.as_ref() {
        distro.as_ref().into()
    } else {
        get_distro_name()
            .context("Failed to get distro name")?
            .into()
    };
    debug!(%distro, "distro name");

    if let Some(distro) = Distro::detect(&distro) {
        return Ok(distro.ascii_art().to_owned());
    }

    todo!()
}

/// Runs neofetch command, returning the piped stdout output.
#[tracing::instrument(level = "debug")]
fn run_neofetch_command_piped<S>(args: &[S]) -> Result<String>
where
    S: AsRef<OsStr> + fmt::Debug,
{
    let mut command = make_neofetch_command(args).context("Failed to make neofetch command")?;

    let output = command
        .output()
        .context("Failed to execute neofetch as child process")?;
    debug!(?output, "neofetch output");

    if !output.status.success() {
        let err = if let Some(code) = output.status.code() {
            anyhow!("neofetch process exited with status code: {code}")
        } else {
            #[cfg(unix)]
            {
                anyhow!(
                    "neofetch process terminated by signal: {}",
                    output
                        .status
                        .signal()
                        .expect("either one of status code or signal should be set")
                )
            }
            #[cfg(not(unix))]
            unimplemented!("status code not expected to be `None` on non-Unix platforms")
        };
        Err(err)?;
    }

    let out = String::from_utf8(output.stdout)
        .context("Failed to process neofetch output as it contains invalid UTF-8")?
        .trim()
        .to_owned();
    Ok(out)
}

fn make_neofetch_command<S>(args: &[S]) -> Result<Command>
where
    S: AsRef<OsStr>,
{
    #[cfg(not(windows))]
    {
        let mut command = Command::new("bash");
        command.arg(get_command_path().context("Failed to get neofetch command path")?);
        command.args(args);
        Ok(command)
    }
    #[cfg(windows)]
    {
        todo!()
    }
}

#[tracing::instrument(level = "debug")]
fn get_distro_name() -> Result<String> {
    run_neofetch_command_piped(&["ascii_distro_name"])
        .context("Failed to get distro name from neofetch")
}