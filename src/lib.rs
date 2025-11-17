mod environment; // environment struct and logic
mod error; // package error handling
mod opt;
mod output; // managing the output of kla
mod reqwest;
mod sigv4;
mod template;
mod tera; // templating responses
mod url_builder;

use std::env;

pub use config::*;
pub use environment::*;
pub use error::*;
pub use opt::*;
pub use output::*;
pub use reqwest::*;
pub use sigv4::*;
pub use template::*;
pub use tera::*;
pub use url_builder::*;

// extending the functionality of our dependancies
pub mod clap;
pub mod config;

// This trait does some string interpilation to turn paths into
// more useful paths
pub trait Expand {
    fn shell_expansion(self) -> String;
}

impl Expand for &str {
    // Does the following
    // replaces ~ with the home directory
    fn shell_expansion(self) -> String {
        self.replace(
            "~",
            env::home_dir()
                .map(|b| b.to_string_lossy().to_string())
                .unwrap_or(String::from("~"))
                .as_str(),
        )
    }
}

impl Expand for String {
    // Does the following
    // replaces ~ with the home directory
    fn shell_expansion(self) -> String {
        self.as_str().shell_expansion()
    }
}

impl Expand for &String {
    // Does the following
    // replaces ~ with the home directory
    fn shell_expansion(self) -> String {
        self.as_str().shell_expansion()
    }
}
