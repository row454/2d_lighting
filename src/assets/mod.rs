use crate::{
    texture::{Texture, TextureCreator},
    texture_atlas::TextureAtlas,
};
use anyhow::{anyhow, Result};
use std::{borrow::Borrow, collections::HashMap, hash::Hash, rc::Rc};

const ASSETS_LOCATION: &str = "assets/";
pub struct AssetStorage<K, R>
where
    K: Hash + Eq,
{
    cache: HashMap<K, Rc<R>>,
}
impl<K, R> AssetStorage<K, R>
where
    K: Hash + Eq,
{
    pub fn new() -> Self {
        AssetStorage {
            cache: HashMap::new(),
        }
    }

    pub fn load<'a, D, L>(&mut self, details: &D, loader: &'a L) -> Result<Rc<R>>
    where
        L: AssetLoader<'a, R, Args = D>,
        D: Eq + Hash + ?Sized,
        K: Borrow<D> + for<'b> From<&'b D>,
    {
        if let Some(resource) = self.cache.get(details).cloned() {
            return Ok(resource);
        }
        let resource = Rc::new(loader.load(details)?);
        self.cache.insert(details.into(), resource.clone());
        Ok(resource)
    }
}

pub type TextureAtlasStorage = AssetStorage<String, TextureAtlas>;

pub trait AssetLoader<'a, R> {
    type Args: ?Sized;
    fn load(&self, data: &Self::Args) -> Result<R>;
}

impl<'a> AssetLoader<'a, Texture> for TextureCreator<'a> {
    type Args = str;

    fn load(&self, data: &Self::Args) -> Result<Texture> {
        self.load(ASSETS_LOCATION.to_owned() + "/textures/" + data + ".png")
    }
}

impl<'a> AssetLoader<'a, TextureAtlas> for TextureCreator<'a> {
    type Args = str;

    fn load(&self, data: &Self::Args) -> Result<TextureAtlas> {
        if let Ok(image) = self.load(ASSETS_LOCATION.to_owned() + "/textures/sheet_" + data + ".png") {
            TextureAtlas::load(
                image,
                ASSETS_LOCATION.to_owned() + "/textures/" + data + ".json",
            )
            .map_err(|e| anyhow!(e))
        } else {
            let image = self.load(ASSETS_LOCATION.to_owned() + "/textures/" + data + ".png")?;
            TextureAtlas::load(
                image,
                ASSETS_LOCATION.to_owned() + "/textures/" + data + ".json",
            )
            .map_err(|e| anyhow!(e))
        }
    }
}
