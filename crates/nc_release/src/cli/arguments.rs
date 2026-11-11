use std::path::PathBuf;

use crate::error::ReleaseError;
use crate::manifest::{FileSpec, parse_file_spec};

#[derive(Debug, Clone)]
pub enum Command {
    Keygen {
        out: PathBuf,
    },
    Manifest {
        channel: String,
        commit: String,
        commit_time: u64,
        files: Vec<FileSpec>,
        out: PathBuf,
    },
    Sign {
        key_file: PathBuf,
        manifest: PathBuf,
    },
}

pub fn parse_command(arguments: &[String]) -> Result<Command, ReleaseError> {
    let (name, rest) = arguments.split_first().ok_or_else(usage)?;
    let flags = Flags::split(rest)?;
    match name.as_str() {
        "keygen" => Ok(Command::Keygen {
            out: flags.one("--out")?.into(),
        }),
        "manifest" => Ok(Command::Manifest {
            channel: flags.one("--channel")?,
            commit: flags.one("--commit")?,
            commit_time: flags
                .one("--commit-time")?
                .parse()
                .map_err(|_| ReleaseError::Usage("--commit-time is not a number".to_owned()))?,
            files: flags
                .all("--file")
                .iter()
                .map(|spec| parse_file_spec(spec))
                .collect::<Result<_, _>>()?,
            out: flags.one("--out")?.into(),
        }),
        "sign" => Ok(Command::Sign {
            key_file: flags.one("--key-file")?.into(),
            manifest: flags.positional()?.into(),
        }),
        _ => Err(usage()),
    }
}

struct Flags {
    named: Vec<(String, String)>,
    positional: Vec<String>,
}

impl Flags {
    fn split(arguments: &[String]) -> Result<Self, ReleaseError> {
        let mut named = Vec::new();
        let mut positional = Vec::new();
        let mut iterator = arguments.iter();
        while let Some(argument) = iterator.next() {
            if argument.starts_with("--") {
                let value = iterator
                    .next()
                    .ok_or_else(|| ReleaseError::Usage(format!("{argument} needs a value")))?;
                named.push((argument.clone(), value.clone()));
            } else {
                positional.push(argument.clone());
            }
        }
        Ok(Self { named, positional })
    }

    fn all(&self, flag: &str) -> Vec<String> {
        self.named
            .iter()
            .filter(|(name, _)| name == flag)
            .map(|(_, value)| value.clone())
            .collect()
    }

    fn one(&self, flag: &str) -> Result<String, ReleaseError> {
        self.all(flag)
            .into_iter()
            .next()
            .ok_or_else(|| ReleaseError::Usage(format!("missing {flag}")))
    }

    fn positional(&self) -> Result<String, ReleaseError> {
        match self.positional.as_slice() {
            [only] => Ok(only.clone()),
            _ => Err(ReleaseError::Usage(
                "expected exactly one manifest path".to_owned(),
            )),
        }
    }
}

fn usage() -> ReleaseError {
    ReleaseError::Usage(
        "usage: nc_release keygen --out <file> | manifest --channel C --commit C --commit-time T \
         --file <path>:<launcher|hook|hook@game>:<platform> ... --out <file> | sign --key-file <file> <manifest>"
            .to_owned(),
    )
}
