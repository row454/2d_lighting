use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, RgbaImage};
use serde::Serialize;

/*
   This tool will support converting a sets of png images into texture atlases.

   By default, it will create a texture atlas for each folder in the input directory, files in the input directory will be ignored. Subfolders will have a texture atlas created for them, which will be placed in the parent folder's atlas.

   This can be changed, so that all files and subfolders in the input directory will be placed in the same atlas, or that only subfolders will have their own atlas.

   The file names will be used as a key in a json; the value will be the texture's rectangle specified as x, y, width, height.

   The output directory will consist of a json file for each atlas, and a png file for each atlas.



   The texture atlas itself contains Regions, which can represent a single texture, an animation, or another texture atlas. An animation can consist of a Vec of textures, animations, or atlases


*/

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    println!("Reading input folder: {}", config.input_folder);

    let sub_directories = get_sub_directories(&config.input_folder)?;
    for sub_directory in sub_directories {
        println!("Found sub directory: {}", sub_directory.display());
        let atlas = create_atlas(&sub_directory, false, &config.output_folder)?;
        if config.pretty {
            serde_json::to_writer_pretty(
                &File::create(
                    [
                        &config.output_folder,
                        &(sub_directory
                            .file_name()
                            .expect("folders must have names")
                            .to_str()
                            .expect("use utf file names")
                            .to_string()
                            + ".json"),
                    ]
                    .iter()
                    .collect::<PathBuf>(),
                )?,
                &atlas,
            )?;
        } else {
            serde_json::to_writer(
                &File::create(
                    [
                        &config.output_folder,
                        &(sub_directory
                            .file_name()
                            .expect("folders must have names")
                            .to_str()
                            .expect("use utf file names")
                            .to_string()
                            + ".json"),
                    ]
                    .iter()
                    .collect::<PathBuf>(),
                )?,
                &atlas,
            )?;
        }
    }
    Ok(())
}
pub struct Config {
    pub input_folder: String,
    pub output_folder: String,
    pub pretty: bool,
}
impl Config {
    pub fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("Not enough arguments");
        }
        let input_folder = args[1].clone();
        let output_folder = args[2].clone();
        let pretty = args
            .get(3)
            .unwrap_or(&"true".to_string())
            .clone()
            .parse::<bool>()
            .map_err(|_| "3rd option should be true or false")?;

        if !Path::new(&input_folder).is_dir() || !Path::new(&output_folder).is_dir() {
            if Path::new(&input_folder).is_dir()
                && !Path::new(&output_folder)
                    .try_exists()
                    .map_err(|_| "Could not check if output location exists!")?
            {
                std::fs::create_dir(&output_folder).map_err(|_| "Could not create dir!")?;
            } else {
                return Err("Input and output folders must be directories");
            }
        }
        Ok(Config {
            input_folder,
            output_folder,
            pretty,
        })
    }
}

fn get_sub_directories<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    Ok(glob::glob(
        format!(
            "{}/*",
            path.as_ref()
                .to_str()
                .expect("use utf file names")
                .trim_end_matches('/')
        )
        .as_str(),
    )?
    .filter_map(|e| e.ok())
    .filter(|e| e.as_path().is_dir())
    .collect::<Vec<_>>())
}
#[derive(Debug, Serialize)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Rect {
    fn new(x: u32, y: u32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}
impl<T: GenericImageView> From<T> for Rect {
    fn from(value: T) -> Self {
        let bounds = value.bounds();
        Rect {
            x: bounds.0,
            y: bounds.1,
            width: bounds.2,
            height: bounds.3,
        }
    }
}

#[derive(Debug, Serialize)]
enum Region {
    Single(Rect),
    NormalPair(Rect, Rect),
    Animation(Rect, Vec<Region>),
    Atlas(Rect, HashMap<String, Region>),
}

struct Strip {
    y: u32,
    height: u32,
    used_width: u32,
}

fn create_atlas<P1: AsRef<Path>, P2: AsRef<Path>>(
    input_folder: P1,
    crop_output: bool,
    output_folder: P2,
) -> Result<HashMap<String, Region>, Box<dyn Error>> {
    println!("{}", input_folder.as_ref().display());
    let mut regions: HashMap<String, Region> = HashMap::new();

    let subfolders = get_sub_directories(&input_folder)?;
    let mut animations = HashMap::<String, Vec<Region>>::new();
    let mut atlases = HashMap::<String, HashMap<String, Region>>::new();
    for folder in subfolders {
        if folder
            .file_stem()
            .unwrap_or_default()
            .to_str()
            .expect("use utf file names")
            .contains("anim")
        {
            animations.insert(
                folder
                    .file_stem()
                    .unwrap_or_default()
                    .to_str()
                    .expect("use utf file names")
                    .to_string(),
                create_animation(folder)?,
            );
        } else {
            atlases.insert(
                String::from("sheet_")
                    + folder
                        .file_stem()
                        .unwrap_or_default()
                        .to_str()
                        .expect("use utf file names"),
                create_atlas(folder, true, &input_folder)?,
            );
        }
    }

    let mut normal_pairs =
        glob::glob(format!("{}/*.png", input_folder.as_ref().display()).as_str())?
            .filter_map(|e| e.ok())
            .filter_map(|path| {
                let mut normal_path = path.clone();
                normal_path.set_file_name(
                    path.file_stem().unwrap().to_string_lossy().to_string() + "_normal.png",
                );
                if normal_path.exists() {
                    Some((path, normal_path))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

    for (name, color, normal) in normal_pairs.iter().filter_map(|paths| {
        if let (Ok(color), Ok(normal)) = (image::open(&paths.0), image::open(&paths.1)) {
            Some((
                paths
                    .0
                    .file_stem()
                    .unwrap_or_default()
                    .to_str()
                    .expect("use utf file names")
                    .to_string(),
                color,
                normal,
            ))
        } else {
            None
        }
    }) {
        if color.dimensions() != normal.dimensions() {
            return Err("Normal map dimensions should match its pair's dimensions!".into());
        }
        let mut buffer = ImageBuffer::new(color.width(), color.height() * 2);
        buffer.copy_from(&color, 0, 0)?;
        buffer.copy_from(&normal, 0, color.height())?;
        buffer.save(
            PathBuf::from(input_folder.as_ref())
                .join(String::from("normal_pair_") + &name + ".png"),
        )?;
    }

    let mut images = glob::glob(format!("{}/*.png", input_folder.as_ref().display()).as_str())?
        .filter_map(|e| e.ok())
        .filter(|x| {
            for pair in normal_pairs.iter() {
                if *x == pair.0 || *x == pair.1 {
                    return false;
                }
            }
            true
        })
        .filter_map(|file: PathBuf| {
            if let Ok(img) = image::open(&file) {
                Some((
                    file.file_stem()
                        .unwrap_or_default()
                        .to_str()
                        .expect("use utf file names")
                        .to_string(),
                    img,
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    images.sort_by_key(|img| (img.1.height(), img.1.width()));
    images.reverse();

    fn add_image(
        name: String,
        image: DynamicImage,
        atlas: &mut RgbaImage,
        regions: &mut HashMap<String, Region>,
        strips: &mut Vec<Strip>,
        atlases: &mut HashMap<String, HashMap<String, Region>>,
        animations: &mut HashMap<String, Vec<Region>>,
    ) -> Result<(), Box<dyn Error>> {
        println!("adding: {}", name);
        for strip in strips.iter_mut() {
            if strip.height < image.height() {
                continue;
            }
            if strip.used_width + image.width() <= atlas.width() {
                if name.starts_with("anim_") {
                    regions.insert(
                        name.strip_prefix("anim_").unwrap().to_string(),
                        Region::Animation(
                            Rect::new(strip.used_width, strip.y, image.width(), image.height()),
                            animations.remove(&name).expect(
                                "images starting with anim_ should only be made by this tool",
                            ),
                        ),
                    );
                } else if name.starts_with("sheet_") {
                    regions.insert(
                        name.strip_prefix("sheet_").unwrap().to_string(),
                        Region::Atlas(
                            Rect::new(strip.used_width, strip.y, image.width(), image.height()),
                            atlases.remove(&name).expect(
                                "images starting with sheet_ should only be made by this tool",
                            ),
                        ),
                    );
                } else if name.starts_with("normal_pair_") {
                    regions.insert(
                        name.strip_prefix("normal_pair_").unwrap().to_string(),
                        Region::NormalPair(
                            Rect::new(strip.used_width, strip.y, image.width(), image.height() / 2),
                            Rect::new(
                                strip.used_width,
                                strip.y + image.height() / 2,
                                image.width(),
                                image.height() / 2,
                            ),
                        ),
                    );
                } else {
                    regions.insert(
                        name.clone(),
                        Region::Single(Rect::new(
                            strip.used_width,
                            strip.y,
                            image.width(),
                            image.height(),
                        )),
                    );
                }
                atlas.copy_from(&image, strip.used_width, strip.y)?;
                strip.used_width += image.width();
                println!("added");
                return Ok(());
            }
        }
        fn add_strip<'a>(strips: &'a mut Vec<Strip>, image: &DynamicImage) -> &'a mut Strip {
            strips.push(Strip {
                y: strips[strips.len() - 1].y + strips[strips.len() - 1].height,
                height: image.height(),
                used_width: 0,
            });
            strips
                .last_mut()
                .expect("this should be initialized with 1 strip")
        }
        let latest_strip = strips
            .last()
            .expect("this should be initialized with 1 strip");
        if atlas.height() >= latest_strip.y + latest_strip.height + image.height() {
            let strip = add_strip(strips, &image);
            if strip.used_width + image.width() <= atlas.width() {
                if name.contains("anim") {
                    regions.insert(
                        name.clone(),
                        Region::Animation(
                            Rect::new(strip.used_width, strip.y, image.width(), image.height()),
                            animations
                                .remove(&name)
                                .expect("images containing anim should only be made by this tool"),
                        ),
                    );
                } else if name.contains("sheet") {
                    regions.insert(
                        name.clone(),
                        Region::Atlas(
                            Rect::new(strip.used_width, strip.y, image.width(), image.height()),
                            atlases
                                .remove(&name)
                                .expect("images containing sheet should only be made by this tool"),
                        ),
                    );
                } else {
                    regions.insert(
                        name.clone(),
                        Region::Single(Rect::new(
                            strip.used_width,
                            strip.y,
                            image.width(),
                            image.height(),
                        )),
                    );
                }
                atlas.copy_from(&image, strip.used_width, strip.y)?;
                strip.used_width += image.width();
                println!("added");
                Ok(())
            } else {
                expand_region(atlas);
                add_image(name, image, atlas, regions, strips, atlases, animations)
            }
        } else {
            expand_region(atlas);
            add_image(name, image, atlas, regions, strips, atlases, animations)
        }
    }
    let mut atlas: RgbaImage = ImageBuffer::new(
        images[0].1.width().next_power_of_two(),
        images[0].1.height().next_power_of_two(),
    );

    let mut strips = vec![Strip {
        y: 0,
        height: images[0].1.height(),
        used_width: 0,
    }];

    for (name, image) in images {
        add_image(
            name,
            image,
            &mut atlas,
            &mut regions,
            &mut strips,
            &mut atlases,
            &mut animations,
        )?;
    }

    if crop_output {
        println!("cropping {}", input_folder.as_ref().display());
        let mut target_width = atlas.width();
        let mut target_height = atlas.height();
        'find_width: while target_width > 0 {
            for y in 0..atlas.height() {
                if atlas.get_pixel(target_width - 1, y)[3] != 0 {
                    break 'find_width;
                }
            }
            target_width -= 1;
        }
        'find_height: while target_height > 0 {
            for x in 0..target_width {
                if atlas.get_pixel(x, target_height - 1)[3] != 0 {
                    break 'find_height;
                }
            }
            target_height -= 1;
        }
        println!("dimensions: {}x{}", target_width, target_height);
        let mut new_atlas: RgbaImage = ImageBuffer::new(target_width, target_height);
        new_atlas.copy_from(
            &(atlas.view(0, 0, target_width, target_height).to_image()),
            0,
            0,
        )?;
        atlas = new_atlas;
    }
    atlas.save(
        output_folder.as_ref().join(PathBuf::from(
            String::from("sheet_")
                + input_folder
                    .as_ref()
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .expect("use utf file names")
                + ".png",
        )),
    )?;

    Ok(regions)
}
enum ExpandDirection {
    Width,
    Height,
}
fn expand_region(region: &mut RgbaImage) -> ExpandDirection {
    let mut new_region: RgbaImage;
    if region.height() < region.width() {
        new_region = ImageBuffer::new(region.width(), (region.height() + 1).next_power_of_two());
        new_region.copy_from(region, 0, 0).unwrap();

        *region = new_region;

        ExpandDirection::Height
    } else {
        new_region = ImageBuffer::new((region.width() + 1).next_power_of_two(), region.height());
        new_region.copy_from(region, 0, 0).unwrap();

        *region = new_region;
        ExpandDirection::Width
    }
}
fn create_animation<P: AsRef<Path>>(folder: P) -> Result<Vec<Region>, Box<dyn Error>> {
    let subfolders = get_sub_directories(&folder)?;
    let mut animations = HashMap::<String, Vec<Region>>::new();
    let mut atlases = HashMap::<String, HashMap<String, Region>>::new();
    for folder in subfolders {
        if folder
            .file_stem()
            .unwrap_or_default()
            .to_str()
            .expect("use utf file names")
            .contains("anim")
        {
            animations.insert(
                folder
                    .file_stem()
                    .unwrap_or_default()
                    .to_str()
                    .expect("use utf file names")
                    .to_string(),
                create_animation(&folder)?,
            );
        } else {
            atlases.insert(
                String::from("sheet_")
                    + folder
                        .file_stem()
                        .unwrap_or_default()
                        .to_str()
                        .expect("use utf file names"),
                create_atlas(&folder, true, &folder)?,
            );
        }
    }

    let mut images = glob::glob(format!("{}/*.png", folder.as_ref().display()).as_str())?
        .filter_map(|e| e.ok())
        .filter_map(|file: PathBuf| {
            if let Ok(img) = image::open(&file) {
                Some((
                    file.file_stem()
                        .unwrap_or_default()
                        .to_str()
                        .expect("use utf file names")
                        .to_string(),
                    img,
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    images.sort_by_key(|img| (img.1.height(), img.1.width()));

    let mut named_regions = Vec::<(String, Region)>::new();
    fn add_image(
        name: String,
        image: DynamicImage,
        animation: &mut RgbaImage,
        regions: &mut Vec<(String, Region)>,
        strips: &mut Vec<Strip>,
        atlases: &mut HashMap<String, HashMap<String, Region>>,
        animations: &mut HashMap<String, Vec<Region>>,
    ) -> Result<(), Box<dyn Error>> {
        for strip in strips.iter_mut() {
            if strip.height < image.height() {
                continue;
            }
            if strip.used_width + image.width() <= animation.width() {
                if name.contains("anim") {
                    regions.push((
                        name.clone(),
                        Region::Animation(
                            Rect::new(strip.used_width, strip.y, image.width(), image.height()),
                            animations
                                .remove(&name)
                                .expect("images containing anim should only be made by this tool"),
                        ),
                    ));
                } else if name.contains("sheet") {
                    regions.push((
                        name.clone(),
                        Region::Atlas(
                            Rect::new(strip.used_width, strip.y, image.width(), image.height()),
                            atlases
                                .remove(&name)
                                .expect("images containing sheet should only be made by this tool"),
                        ),
                    ));
                } else {
                    regions.push((
                        name.clone(),
                        Region::Single(Rect::new(
                            strip.used_width,
                            strip.y,
                            image.width(),
                            image.height(),
                        )),
                    ));
                }
                animation.copy_from(&image, strip.used_width, strip.y)?;
                strip.used_width += image.width();
                return Ok(());
            }
        }
        fn add_strip<'a>(strips: &'a mut Vec<Strip>, image: &DynamicImage) -> &'a mut Strip {
            strips.push(Strip {
                y: strips[strips.len() - 1].y + strips[strips.len() - 1].height,
                height: image.height(),
                used_width: 0,
            });
            strips
                .last_mut()
                .expect("this should be initialized with 1 strip")
        }
        let latest_strip = strips
            .last()
            .expect("this should be initialized with 1 strip");
        if animation.height() >= latest_strip.y + latest_strip.height + image.height() {
            let strip = add_strip(strips, &image);
            if strip.used_width + image.width() <= animation.width() {
                if name.contains("anim") {
                    regions.push((
                        name.clone(),
                        Region::Animation(
                            Rect::new(strip.used_width, strip.y, image.width(), image.height()),
                            animations
                                .remove(&name)
                                .expect("images containing anim should only be made by this tool"),
                        ),
                    ));
                } else if name.contains("sheet") {
                    regions.push((
                        name.clone(),
                        Region::Atlas(
                            Rect::new(strip.used_width, strip.y, image.width(), image.height()),
                            atlases
                                .remove(&name)
                                .expect("images containing sheet should only be made by this tool"),
                        ),
                    ));
                } else {
                    regions.push((
                        name.clone(),
                        Region::Single(Rect::new(
                            strip.used_width,
                            strip.y,
                            image.width(),
                            image.height(),
                        )),
                    ));
                }
                animation.copy_from(&image, strip.used_width, strip.y)?;
                strip.used_width += image.width();
                Ok(())
            } else {
                expand_region(animation);
                add_image(name, image, animation, regions, strips, atlases, animations)
            }
        } else {
            expand_region(animation);
            add_image(name, image, animation, regions, strips, atlases, animations)
        }
    }
    let mut animation: RgbaImage = ImageBuffer::new(
        images[0].1.width().next_power_of_two(),
        images[0].1.height().next_power_of_two(),
    );

    let mut strips = vec![Strip {
        y: 0,
        height: images[0].1.height(),
        used_width: 0,
    }];

    for (name, image) in images {
        add_image(
            name,
            image,
            &mut animation,
            &mut named_regions,
            &mut strips,
            &mut atlases,
            &mut animations,
        )?;
    }

    animation.save(
        folder.as_ref().parent().unwrap().join(PathBuf::from(
            String::from("anim_")
                + folder
                    .as_ref()
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .expect("use utf file names")
                + ".png",
        )),
    )?;

    let number_regex = regex::Regex::new("[0-9]+").unwrap();
    named_regions.sort_by_key(|(name, _img)| {
        number_regex
            .find(name)
            .expect("animation frames must be numbered")
            .as_str()
            .parse::<u32>()
            .expect("animation frames must be numbered")
    });
    let (_names, regions): (Vec<String>, _) = named_regions.into_iter().unzip();
    Ok(regions)
}
