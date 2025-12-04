use std::{fs, io, pin::Pin};

use anyhow::Context;
use clap::{
    builder::{IntoResettable, OsStr},
    Arg,
};
use log::debug;
use tokio::{fs::File, io::AsyncWrite};

use crate::{impl_ok, impl_opt, Expand as _};

pub trait DefaultValueIfSome {
    fn default_value_if_some(self, val: Option<impl IntoResettable<OsStr>>) -> Self;
}

impl DefaultValueIfSome for Arg {
    fn default_value_if_some(self, val: Option<impl IntoResettable<OsStr>>) -> Self {
        if let Some(default_env) = val {
            self.default_value(default_env)
        } else {
            self
        }
    }
}

/// arg_file_value makes it possible to read an argument from standard in or a file
/// or just take the literal value. The function checks for a prefix which defines how
/// we treat the incoming value
/// `-` => Read from standard input
/// `@` => Read from file (followed by a path `@/tmp/myfile.txt`)
///
/// else: Take the literal value
pub fn arg_file_value(val: Option<&String>, name: &str) -> Result<Option<String>, anyhow::Error> {
    val.map(|val| match val.chars().nth(0) {
        Some('-') => {
            debug!("read {} from standard in", name);
            io::read_to_string(io::stdin())
        }
        Some('@') => {
            let filename = val
                .strip_prefix("@")
                .expect("the prefix and the matching arm must match for this to work")
                .shell_expansion();
            debug!("reading {} from contents of file {}", name, filename);
            fs::read_to_string(filename)
        }
        _ => {
            debug!("reading {} as literal", name);
            Ok(val.into())
        }
    })
    .transpose()
    .with_context(|| {
        format!(
            "could not read {} from input {}",
            name,
            val.map(|v| v.as_str()).unwrap_or_default()
        )
    })
}

pub async fn arg_file_writer(
    val: Option<&String>,
    name: &str,
) -> Option<Result<Pin<Box<dyn AsyncWrite>>, anyhow::Error>> {
    let val = if let Some(val) = val {
        val
    } else {
        return None;
    };

    let writer = match val.chars().nth(0) {
        Some('-') => {
            debug!("write to standard out");
            Ok(Box::pin(tokio::io::stdout()) as Pin<Box<dyn AsyncWrite>>)
        }
        _ => {
            let name = name.shell_expansion();
            debug!("write to file {}", name);
            File::open(&name)
                .await
                .map::<Pin<Box<dyn AsyncWrite>>, _>(|w| Box::pin(w))
                .with_context(|| format!("The file {} could not be written to", &name))
        }
    };

    Some(writer)
}

impl_opt!(clap::Command, crate::Error);
impl_opt!(clap::Arg, crate::Error);
impl_ok!(clap::Command, crate::Error);
impl_ok!(clap::Arg, crate::Error);
