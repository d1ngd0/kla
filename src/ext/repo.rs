use anyhow::Context;
use flate2::read::GzDecoder;
use oci_client::{client::ClientConfig, secrets::RegistryAuth, Client, Reference};
use tar::Archive;

use crate::{config::Config, ExtensionSet, KResult, EXTENSION_ROOT};
use std::{
    fs::{self},
    hash::{DefaultHasher, Hash, Hasher as _},
    io::Write,
    path::{Path, PathBuf},
};

use super::Extension;

pub struct ExtensionRepo {
    dir: PathBuf,
}

impl ExtensionRepo {
    /// new creates a new repo extension from the following directory
    pub fn new<P: AsRef<Path>>(dir: P) -> KResult<Self> {
        // make sure the directory exists, and return it
        if !dir.as_ref().exists() {
            fs::create_dir_all(dir.as_ref()).with_context(|| {
                format!(
                    "creating extension directory {}",
                    dir.as_ref().to_string_lossy()
                )
            })?;
        }

        Ok(Self {
            dir: PathBuf::from(dir.as_ref()),
        })
    }

    /// oci_client returns an opinionated client to the caller to fetch or push things
    fn oci_client(&self) -> KResult<Client> {
        Ok(Client::new(ClientConfig::default()))
    }

    /// oci_client_auth returns an opinionated authentication mechanism that caller should
    /// use
    fn oci_client_auth(&self) -> KResult<RegistryAuth> {
        Ok(RegistryAuth::Anonymous)
    }

    /// extensions returns the current extensions from the extensions file
    fn extensions(&self) -> KResult<ExtensionSet> {
        let path = self.dir.join(EXTENSION_ROOT);
        if path.exists() {
            let extensions = fs::read_to_string(&path).with_context(|| {
                format!(
                    "reading extension file {}",
                    path.as_path().to_string_lossy()
                )
            })?;
            Ok(toml::from_str(&extensions).with_context(|| {
                format!(
                    "unmarshaling extension file {}",
                    path.as_path().to_string_lossy()
                )
            })?)
        } else {
            Ok(ExtensionSet::empty())
        }
    }

    /// commit_extension adds a single extension by loading the file,
    /// and writing the value.
    fn commit_extension(&self, ext: Extension) -> KResult<()> {
        let mut extensions = self.extensions()?;

        if !extensions.iter().any(|v| v == &ext) {
            extensions.push(ext);
            self.commit_extensions(extensions)
                .context("commiting extensions")?;
        }
        Ok(())
    }

    /// commit_extensions writes extensions back to the file
    fn commit_extensions(&self, extensions: ExtensionSet) -> KResult<()> {
        let path = self.dir.join(EXTENSION_ROOT);
        let extensions = toml::to_string(&extensions).context("marshaling extensions")?;
        Ok(fs::write(path.as_path(), &extensions).with_context(|| {
            format!(
                "writing extensions file {}",
                path.as_path().to_string_lossy()
            )
        })?)
    }

    /// add installs the extension into the repo. If the extension already exists it will
    /// remove it before pulling the latest image and installing it. The extension name
    /// is defined by the registry and repository, so docker.example.com/ceph/admin:latest
    /// would be stored in <extension_directory>/docker.example.com/ceph/admin. Calling this
    /// function with a new `tag` would effectively update the extension.
    pub async fn add<W: Write>(&self, image: &Reference, stdout: &mut W) -> KResult<PathBuf> {
        let ext_dir = self.dir.join(image.registry()).join(image.repository());
        if ext_dir.exists() {
            fs::remove_dir_all(&ext_dir).with_context(|| {
                format!(
                    "removing previous extension directory {}",
                    ext_dir.as_path().to_string_lossy()
                )
            })?;
        }

        let _ = writeln!(stdout, "Pulling {}", image);
        let layers = self
            .oci_client()?
            .pull(
                &image,
                &self.oci_client_auth()?,
                vec!["application/vnd.oci.image.layer.v1.tar+gzip"],
            )
            .await?
            .layers
            .into_iter();

        for layer in layers {
            let mut hasher = DefaultHasher::new();
            layer.hash(&mut hasher);
            let hash = hasher.finish();
            let _ = writeln!(stdout, "unpacking layer <{:x}>", hash);
            let mut layer = Archive::new(GzDecoder::new(layer.data.as_ref()));
            layer
                .unpack(&ext_dir)
                .with_context(|| format!("merging layer {:x}", hash))?
        }

        let _ = write!(
            stdout,
            "installed extension to {}",
            &ext_dir.to_string_lossy()
        );

        self.commit_extension(Extension {
            dir: ext_dir.clone(),
            remote: image.clone(),
        })?;

        Ok(ext_dir)
    }

    pub fn apply(&self, config: &mut Config) -> KResult<()> {
        let extensions = self.extensions().context("fetching extensions")?;
        for extension in extensions.iter() {
            config.merge(extension.try_into().with_context(|| {
                format!("loading extension configuration for {}", &extension.remote)
            })?);
        }
        Ok(())
    }
}
