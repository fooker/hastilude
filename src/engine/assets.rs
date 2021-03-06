use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::fmt;

use anyhow::{anyhow, Context, Result};
use rand::seq::SliceRandom;
use tracing::{trace_span,instrument};

use crate::engine::sound::Music;
use std::fmt::Debug;

pub trait AssetLoader: Sized {
    type Asset;

    fn load(path: impl AsRef<Path>) -> Result<Self::Asset>;
}

pub struct Asset<L: AssetLoader> {
    pub path: PathBuf,
    pub name: String,

    loader: PhantomData<L>,
}

impl<L: AssetLoader> Asset<L> {
    #[instrument(level = "debug", name = "Asset::load")]
    pub fn load(&self) -> L::Asset {
        return trace_span!("Loading asset", path=?self.path)
            .in_scope(|| L::load(&self.path))
            .with_context(|| format!("Loading asset: {:?}", self.path))
            .expect("Failed to load asset");
    }
}

impl<L: AssetLoader> fmt::Debug for Asset<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Asset")
            .field("name", &self.name)
            .field("path", &self.path)
            .finish()
    }
}

#[derive(Debug)]
pub struct AssetBundle<L: AssetLoader> {
    assets: Vec<Asset<L>>,
}

impl<L: AssetLoader> AssetBundle<L> {
    #[instrument(level = "debug")]
    pub(self) fn load(path: impl AsRef<Path> + Debug) -> Result<Self> {
        let assets = path.as_ref().read_dir()
            .with_context(|| format!("Failed to open asset directory: {:?}", path.as_ref()))?
            .map(|entry| {
                let entry = entry?;
                let name = entry.path().file_stem()
                    .ok_or(anyhow!("Invalid filename: {:?}", entry.path()))?
                    .to_string_lossy().to_string();

                return Ok(Asset {
                    path: entry.path(),
                    name,
                    loader: Default::default(),
                });
            })
            .collect::<Result<_>>()?;

        return Ok(Self {
            assets
        });
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item=&Asset<L>> {
        return self.assets.iter();
    }

    pub fn get(&self, name: &str) -> Option<&Asset<L>> {
        return self.assets.iter()
            .find(|asset| asset.name == name);
    }

    pub fn random(&self) -> &Asset<L> {
        return self.assets.choose(&mut rand::thread_rng())
            .expect("Asset not available");
    }
}

pub struct Assets {
    pub music: AssetBundle<Music>,
}

impl Assets {
    #[instrument(level = "debug")]
    pub fn init(path: impl AsRef<Path> + Debug) -> Result<Self> {
        let music = AssetBundle::load(path.as_ref().join("music"))
            .context("Failed to load music assets")?;

        return Ok(Self {
            music,
        });
    }
}