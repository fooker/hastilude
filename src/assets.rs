use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use rand::seq::SliceRandom;
use tracing::trace_span;

use crate::sound::Music;

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
    pub fn load(&self) -> L::Asset {
        return trace_span!("Loading asset", path=?self.path)
            .in_scope(|| L::load(&self.path))
            .with_context(|| format!("Loading asset: {:?}", self.path))
            .expect("Failed to load asset");
    }
}

pub struct AssetBundle<L: AssetLoader> {
    assets: Vec<Asset<L>>,
}

impl<L: AssetLoader> AssetBundle<L> {
    pub(self) fn load(path: impl AsRef<Path>) -> Result<Self> {
        let assets = path.as_ref().read_dir()?
            .map(|entry| {
                let entry = entry?;
                let name = entry.path().file_stem()
                    .ok_or(anyhow!("Invalid filename"))?
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
    pub fn init(path: impl AsRef<Path>) -> Result<Self> {
        let music = AssetBundle::load(path.as_ref().join("music"))?;

        return Ok(Self {
            music,
        });
    }
}