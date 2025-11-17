use clap::{
    builder::{IntoResettable, OsStr},
    Arg,
};

use crate::{impl_ok, impl_opt};

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

impl_opt!(clap::Command, crate::Error);
impl_opt!(clap::Arg, crate::Error);
impl_ok!(clap::Command, crate::Error);
impl_ok!(clap::Arg, crate::Error);
