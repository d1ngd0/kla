/// The Filecache holds onto some data at some specified location until
/// it "expires". At which point it will call the function to generate
/// the value again.
use std::{
    fs::{self, read_to_string},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Clone, Debug)]
pub struct CacheFile {
    location: PathBuf,
    contents: Arc<RwLock<FileContents>>,
}

impl CacheFile {
    pub fn new<P>(location: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            location: PathBuf::from(location.as_ref()),
            contents: Arc::new(RwLock::new(FileContents {
                content: String::new(),
                expires: Local::now(),
            })),
        }
    }

    pub fn fetch<Fn>(&self, constructor: Fn) -> Result<String>
    where
        Fn: FnOnce() -> Result<(String, chrono::Duration)>,
    {
        // First check if the contents are in memory, if they are and aren't expired let's return that
        let contents = self.contents.read().unwrap();
        if let Some(data) = contents.fetch() {
            return Ok(data.clone());
        }
        drop(contents);

        // The reading stuff is done, we want to make sure we are the only ones doing things
        // now, so lets get to work by fetching a write lock
        let mut contents = self.contents.write().unwrap();

        // it's possible more than one of us got to this point, so let's make sure
        // a sibling process didn't already update the contents, if they did we
        // can return and unlock
        if let Some(data) = contents.fetch() {
            return Ok(data.clone());
        }

        // Alright, the contents are not in memory, or the data is old, let's try
        // to fetch it from the disk. First check if the file exists, if it does
        // we can load it into memory, and return the value
        if self.location.exists() {
            let file_contents = read_to_string(&self.location)?;
            let file_contents: FileContents = serde_json::from_str(&file_contents.as_str())?;

            if !file_contents.expired() {
                contents.update(file_contents);
                return Ok(contents
                    .fetch()
                    .expect("We litterally just set the token and it wasn't expired")
                    .clone());
            }
        }

        // Ok the path doesn't exist, so we need to call the function to create
        // the contents
        let process_contents = constructor()?;
        fs::write(&self.location, serde_json::to_string(&process_contents)?)?;

        contents.update(FileContents {
            content: process_contents.0,
            expires: Local::now() + process_contents.1,
        });

        return Ok(contents
            .fetch()
            .expect("We litterally just set the token and it wasn't expired")
            .clone());
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileContents {
    content: String,
    expires: DateTime<Local>,
}

impl FileContents {
    /// empty returns if the token has been set or not
    pub fn empty(&self) -> bool {
        return self.content.as_str() == "";
    }

    /// expired returns true when the token is expired
    pub fn expired(&self) -> bool {
        self.expires <= Local::now()
    }

    /// update consumes a TokenFileContents and sets its
    /// values to the values specified in from.
    pub fn update(&mut self, from: Self) {
        self.content = from.content;
        self.expires = from.expires
    }

    /// token returns a reference to the AccessToken if it
    /// is set and not expired, otherwise it returns None.
    pub fn fetch(&self) -> Option<&String> {
        if self.empty() {
            return None;
        }

        if self.expired() {
            return None;
        }

        Some(&self.content)
    }
}
