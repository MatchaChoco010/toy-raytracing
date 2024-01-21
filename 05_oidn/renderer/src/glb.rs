mod scene;
mod utils;

use std::error::Error;
use std::path::Path;

pub use scene::*;
use utils::GlbData;

pub fn load<P>(path: P) -> Result<Vec<Scene>, Box<dyn Error + Send + Sync>>
where
    P: AsRef<Path>,
{
    let (doc, buffers, images) = gltf::import(&path)?;

    let mut data = GlbData::new(buffers, images, &path);

    let mut res = vec![];
    for scene in doc.scenes() {
        res.push(Scene::load(scene, &mut data));
    }
    Ok(res)
}
