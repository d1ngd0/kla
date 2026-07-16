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
/// `@` => Read from file (followed by a path `@/tmp/myfile.txt`), if the file doesn't
///        exist we return None
/// `!` => Read from file (followed by a path `!/tmp/myfile.txt`), if the files doesn't
///        exists return an error
/// ``
///
/// else: Take the literal value
///
/// > [!IMPORTANT]
/// This should only be avaialble int he command line, file paths inside templates should
/// be relative to the file they are defined in.
pub fn arg_file_value(val: Option<&String>, name: &str) -> Result<Option<String>, anyhow::Error> {
    let val = if let Some(val) = val {
        val
    } else {
        return Ok(None);
    };

    let val = match val.chars().nth(0) {
        Some('-') => {
            debug!("read {} from standard in", name);
            Some(
                io::read_to_string(io::stdin())
                    .with_context(|| format!("could not read {} from standard in", name))?,
            )
        }
        Some('@') => {
            let filename = val
                .strip_prefix("@")
                .expect("the prefix and the matching arm must match for this to work")
                .shell_expansion();
            debug!("reading {} from contents of file {}", name, filename);
            Some(fs::read_to_string(filename.as_str()).with_context(|| {
                format!("could not read {} from file {}", name, filename.as_str())
            })?)
        }
        Some('?') => {
            let filename = val
                .strip_prefix("?")
                .expect("the prefix and the matching arm must match for this to work")
                .shell_expansion();
            debug!(
                "reading {} from contents of file {}",
                name,
                filename.as_str()
            );
            fs::read_to_string(filename.as_str()).ok()
        }
        _ => {
            debug!("reading {} as literal", name);
            Some(val.into())
        }
    };

    Ok(val)
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
            debug!("writing {} standard out", name);
            Ok(Box::pin(tokio::io::stdout()) as Pin<Box<dyn AsyncWrite>>)
        }
        _ => {
            let val = val.shell_expansion();
            debug!("writing {} to file {}", name, val);
            File::create(&val)
                .await
                .map::<Pin<Box<dyn AsyncWrite>>, _>(|w| Box::pin(w))
                .with_context(|| format!("The file {} could not be written to", &val))
        }
    };

    Some(writer)
}

impl_opt!(clap::Command, crate::Error);
impl_opt!(clap::Arg, crate::Error);
impl_ok!(clap::Command, crate::Error);
impl_ok!(clap::Arg, crate::Error);
