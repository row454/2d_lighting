use std::{collections::HashMap, fs::File, io::BufReader, path::Path, rc::Rc, sync::Arc};

use serde::Deserialize;

use crate::texture::Texture;

#[derive(Clone)]
pub struct TextureRegion {
    pub texture: Arc<Texture>,
    pub src: Rect,
}

impl std::fmt::Debug for TextureRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("TextureRegion")
            .field("src", &self.src)
            .finish()
    }
}

#[derive(Clone)]
pub struct DeferredTextureRegion {
    pub texture: Arc<Texture>,
    pub albedo: Rect,
    pub normal: Rect,
}
impl std::fmt::Debug for DeferredTextureRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("NormalPairTextureRegion")
            .field("albedo", &self.albedo)
            .field("normal", &self.normal)
            .finish()
    }
}

pub struct TextureAtlas {
    image: Arc<Texture>,
    regions: HashMap<String, Rc<Region>>,
}

impl TextureAtlas {
    pub fn new(image: Arc<Texture>) -> Self {
        TextureAtlas {
            image,
            regions: HashMap::new(),
        }
    }
    pub fn load<P: AsRef<Path>>(image: Texture, atlas_json: P) -> Result<TextureAtlas, String> {
        let mut atlas = TextureAtlas::new(Arc::new(image));
        let raw_regions: HashMap<String, RawRegion> = serde_json::from_reader(BufReader::new(
            File::open(atlas_json).map_err(|e| e.to_string())?,
        ))
        .map_err(|e| e.to_string())?;

        for (name, region) in raw_regions {
            atlas
                .regions
                .insert(name, Rc::new(region.set_image(atlas.image.clone(), 0, 0)));
        }

        println!("{:?}", atlas.regions);
        Ok(atlas)
    }
    pub fn get_region(&self, details: &str) -> Option<Rc<Region>> {
        self.regions.get(details).cloned()
    }
}
#[derive(Deserialize)]
enum RawRegion {
    Single(Rect),
    NormalPair(Rect, Rect),
    Animation(Rect, Vec<RawRegion>),
    Atlas(Rect, HashMap<String, RawRegion>),
}

impl RawRegion {
    fn set_image(self, texture: Arc<Texture>, x_offset: u32, y_offset: u32) -> Region {
        match self {
            Self::Single(mut src) => Region::Single({
                src.x += x_offset;
                src.y += y_offset;

                TextureRegion { texture, src }
            }),
            Self::NormalPair(mut albedo, mut normal) => Region::NormalPair({
                albedo.x += x_offset;
                albedo.y += y_offset;
                normal.x += x_offset;
                normal.y += y_offset;

                DeferredTextureRegion {
                    texture,
                    albedo,
                    normal,
                }
            }),
            Self::Animation(mut src, raw_frames) => {
                src.x += x_offset;
                src.y += y_offset;
                let mut frames = Vec::new();
                for frame in raw_frames {
                    frames.push(frame.set_image(texture.clone(), src.x, src.y));
                }

                Region::Animation(frames)
            }
            Self::Atlas(mut src, raw_atlas) => {
                src.x += x_offset;
                src.y += y_offset;
                let mut atlas = HashMap::new();
                for (name, region) in raw_atlas {
                    atlas.insert(
                        name,
                        region.set_image(texture.clone(), x_offset + src.x, y_offset + src.y),
                    );
                }

                Region::Atlas(atlas)
            }
        }
    }
}
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub enum Region {
    Single(TextureRegion),
    NormalPair(DeferredTextureRegion),
    Animation(Vec<Region>),
    Atlas(HashMap<String, Region>),
}

#[allow(dead_code)]
impl Region {
    pub fn expect_single(&self, reason: &'static str) -> TextureRegion {
        if let Self::Single(region) = self {
            region.to_owned()
        } else {
            panic!("{reason}: {self:?}");
        }
    }
    pub fn expect_pair(&self, reason: &'static str) -> DeferredTextureRegion {
        if let Self::NormalPair(region) = self {
            region.to_owned()
        } else {
            panic!("{reason}: {self:?}");
        }
    }
    pub fn expect_animation(&self, reason: &'static str) -> Vec<Region> {
        if let Self::Animation(frames) = self {
            frames.to_owned()
        } else {
            panic!("{reason}: {self:?}");
        }
    }
    pub fn expect_atlas(&self, reason: &'static str) -> HashMap<String, Region> {
        if let Self::Atlas(atlas) = self {
            atlas.to_owned()
        } else {
            panic!("{reason}: {self:?}");
        }
    }
    pub fn unwrap_single(&self) -> TextureRegion {
        if let Self::Single(region) = self {
            region.to_owned()
        } else {
            panic!("unwrap_single failed, was given: {self:?}");
        }
    }
    pub fn unwrap_pair(&self) -> DeferredTextureRegion {
        if let Self::NormalPair(region) = self {
            region.to_owned()
        } else {
            panic!("unwrap_single failed, was given: {self:?}");
        }
    }
    pub fn unwrap_animation(&self) -> Vec<Region> {
        if let Self::Animation(frames) = self {
            frames.to_owned()
        } else {
            panic!("unwrap_animation failed, was given: {self:?}");
        }
    }
    pub fn unwrap_atlas(&self) -> HashMap<String, Region> {
        if let Self::Atlas(atlas) = self {
            atlas.to_owned()
        } else {
            panic!("unwrap_atlas failed, was given: {self:?}");
        }
    }
}
