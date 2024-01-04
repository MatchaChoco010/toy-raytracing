use std::cmp::Ordering;

use image::{ImageBuffer, Rgb};
use rand::distributions::Uniform;
use rand::prelude::*;
use rayon::prelude::*;

struct Sample {
    sample_dir_tangent: glam::Vec3,
    bsdf_multiplied_cos_divided_by_pdf: glam::Vec3,
}

#[derive(Debug, Clone, Copy)]
enum Material {
    Lambert { color: glam::Vec3 },
    Emissive { color: glam::Vec3, strength: f32 },
    Glass { inner_eta: f32, outer_eta: f32 },
}
impl Material {
    fn emissive(&self) -> glam::Vec3 {
        match self {
            Material::Lambert { .. } => glam::Vec3::ZERO,
            Material::Emissive { color, strength } => *color * *strength,
            Material::Glass { .. } => glam::Vec3::ZERO,
        }
    }

    #[allow(dead_code)]
    fn bsdf(
        &self,
        view_dir_tangent: glam::Vec3,
        light_dir_tangent: glam::Vec3,
        front_hit: bool,
    ) -> glam::Vec3 {
        match self {
            Material::Lambert { color } => *color / std::f32::consts::PI,
            Material::Emissive { .. } => glam::Vec3::ZERO,
            Material::Glass {
                inner_eta,
                outer_eta,
            } => {
                let (eta_1, eta_2) = if front_hit {
                    (*outer_eta, *inner_eta)
                } else {
                    (*inner_eta, *outer_eta)
                };

                let refract_dir = (eta_1 / eta_2)
                    * (-view_dir_tangent + view_dir_tangent.dot(glam::Vec3::Y) * glam::Vec3::Y)
                    - (1.0
                        - (eta_1 / eta_2).powi(2)
                            * (1.0 + view_dir_tangent.dot(glam::Vec3::Y).powi(2)))
                    .sqrt()
                        * glam::Vec3::Y;
                let reflect_dir =
                    -2.0 * -view_dir_tangent.dot(glam::Vec3::Y) * glam::Vec3::Y - view_dir_tangent;

                let is_total_internal_reflection = (1.0
                    - (eta_1 / eta_2).powi(2)
                        * (1.0 + view_dir_tangent.dot(glam::Vec3::Y).powi(2)))
                    < 0.0;

                if is_total_internal_reflection {
                    if reflect_dir.dot(light_dir_tangent) > 0.99999 {
                        glam::Vec3::ONE
                    } else {
                        glam::Vec3::ZERO
                    }
                } else {
                    if reflect_dir.dot(light_dir_tangent) > 0.99999
                        || refract_dir.dot(light_dir_tangent) > 0.99999
                    {
                        glam::Vec3::ONE
                    } else {
                        glam::Vec3::ZERO
                    }
                }
            }
        }
    }

    fn sample(
        &self,
        view_dir_tangent: glam::Vec3,
        front_hit: bool,
        mut rng: &mut ThreadRng,
    ) -> Option<Sample> {
        match self {
            Material::Lambert { color, .. } => {
                let uniform = Uniform::new(0.0, 1.0);

                let (u1, u2): (f32, f32) = (uniform.sample(&mut rng), uniform.sample(&mut rng));
                let r = u1.sqrt();
                let phi = 2.0 * std::f32::consts::PI * u2;
                let sample_dir = glam::Vec3::new(r * phi.cos(), (1.0 - u1).sqrt(), r * phi.sin());

                let sample = Sample {
                    sample_dir_tangent: sample_dir,
                    bsdf_multiplied_cos_divided_by_pdf: *color,
                };

                Some(sample)
            }
            Material::Emissive { .. } => None,
            Material::Glass {
                inner_eta,
                outer_eta,
            } => {
                let (eta_1, eta_2) = if front_hit {
                    (*outer_eta, *inner_eta)
                } else {
                    (*inner_eta, *outer_eta)
                };

                let refract_dir = (eta_1 / eta_2)
                    * (-view_dir_tangent + view_dir_tangent.dot(glam::Vec3::Y) * glam::Vec3::Y)
                    - (1.0
                        - (eta_1 / eta_2).powi(2)
                            * (1.0 + view_dir_tangent.dot(glam::Vec3::Y).powi(2)))
                    .sqrt()
                        * glam::Vec3::Y;
                let reflect_dir =
                    -2.0 * -view_dir_tangent.dot(glam::Vec3::Y) * glam::Vec3::Y - view_dir_tangent;

                let is_total_internal_reflection = (1.0
                    - (eta_1 / eta_2).powi(2)
                        * (1.0 + (view_dir_tangent.dot(glam::Vec3::Y)).powi(2)))
                    < 0.0;

                if is_total_internal_reflection {
                    let sample = Sample {
                        sample_dir_tangent: reflect_dir,
                        bsdf_multiplied_cos_divided_by_pdf: glam::Vec3::ONE,
                    };
                    return Some(sample);
                }

                let cos_theta_i = view_dir_tangent.dot(glam::Vec3::Y);
                let cos_theta_o = refract_dir.dot(glam::Vec3::NEG_Y);
                let rho_s = (eta_1 * cos_theta_i - eta_2 * cos_theta_o)
                    / (eta_1 * cos_theta_i + eta_2 * cos_theta_o);
                let rho_p = (eta_1 * cos_theta_o - eta_2 * cos_theta_i)
                    / (eta_1 * cos_theta_o + eta_2 * cos_theta_i);
                let fresnel = (rho_s.powi(2) + rho_p.powi(2)) / 2.0;

                let uniform = Uniform::new(0.0, 1.0);
                if uniform.sample(&mut rng) < fresnel {
                    let sample = Sample {
                        sample_dir_tangent: reflect_dir,
                        bsdf_multiplied_cos_divided_by_pdf: glam::Vec3::ONE,
                    };
                    return Some(sample);
                } else {
                    let sample = Sample {
                        sample_dir_tangent: refract_dir,
                        bsdf_multiplied_cos_divided_by_pdf: glam::Vec3::ONE,
                    };
                    return Some(sample);
                }
            }
        }
    }

    fn russian_roulette_probability(&self) -> f32 {
        match self {
            Material::Lambert { color } => color.x.max(color.y.max(color.z)),
            Material::Emissive { .. } => 1.0,
            Material::Glass { .. } => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Ray {
    origin: glam::Vec3,
    dir: glam::Vec3,
}

#[derive(Debug, Clone, Copy)]
enum AABBHit {
    Miss,
    Hit,
}

#[derive(Debug, Clone, Copy)]
struct AABB {
    min: glam::Vec3,
    max: glam::Vec3,
}
impl AABB {
    fn merge(&self, other: &Self) -> Self {
        let min = self.min.min(other.min);
        let max = self.max.max(other.max);
        Self { min, max }
    }

    fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    fn intersect(&self, ray: &Ray) -> AABBHit {
        let inv_dir = glam::Vec3::new(1.0, 1.0, 1.0) / ray.dir;
        let t1 = (self.min - ray.origin) * inv_dir;
        let t2 = (self.max - ray.origin) * inv_dir;
        let tmin = t1.min(t2);
        let tmax = t1.max(t2);
        let tmin = tmin.max_element();
        let tmax = tmax.min_element();
        if tmin <= tmax && (tmin > 0.0 || tmax > 0.0) {
            AABBHit::Hit
        } else {
            AABBHit::Miss
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TriangleHit {
    Miss,
    Hit {
        t: f32,
        position: glam::Vec3,
        normal: glam::Vec3,
        material: Material,
    },
}

struct Triangle {
    pa: glam::Vec3,
    pb: glam::Vec3,
    pc: glam::Vec3,
    na: glam::Vec3,
    nb: glam::Vec3,
    nc: glam::Vec3,
    material: Material,
}
impl Triangle {
    fn aabb(&self) -> AABB {
        let min = self.pa.min(self.pb).min(self.pc);
        let max = self.pa.max(self.pb).max(self.pc);
        AABB { min, max }
    }

    fn center(&self) -> glam::Vec3 {
        // (self.pa + self.pb + self.pc) / 3.0
        let aabb = self.aabb();
        (aabb.min + aabb.max) / 2.0
    }

    fn intersect(&self, ray: &Ray, tmin: f32, tmax: f32) -> TriangleHit {
        let o = ray.origin;
        let q = ray.origin + ray.dir;

        let vc = (q - o).dot((self.pb - o).cross(self.pa - o));
        let vb = (q - o).dot((self.pa - o).cross(self.pc - o));
        let va = (q - o).dot((self.pc - o).cross(self.pb - o));

        if va <= 0.0 || vb <= 0.0 || vc <= 0.0 {
            return TriangleHit::Miss;
        }

        let v = va + vb + vc;
        let alpha = va / v;
        let beta = vb / v;
        let gamma = vc / v;

        let position = alpha * self.pa + beta * self.pb + gamma * self.pc;

        let t = (position - ray.origin).dot(ray.dir);
        if t < tmin || t > tmax {
            return TriangleHit::Miss;
        }

        let normal = alpha * self.na + beta * self.nb + gamma * self.nc;
        let normal = normal.normalize();

        TriangleHit::Hit {
            t,
            position,
            normal,
            material: self.material,
        }
    }
}

type TriangleList = Vec<Triangle>;
trait TriangleListExtension {
    fn new() -> Self;
    fn add_model(&mut self, model: &tobj::Model, material: Material);
}
impl TriangleListExtension for TriangleList {
    fn new() -> Self {
        vec![]
    }

    fn add_model(&mut self, model: &tobj::Model, material: Material) {
        let mesh = &model.mesh;
        let positions = &mesh.positions;
        let normals = &mesh.normals;
        let indices = &mesh.indices;
        for i in (0..indices.len()).step_by(3).rev() {
            let pa = glam::Vec3::new(
                positions[indices[i] as usize * 3],
                positions[indices[i] as usize * 3 + 1],
                positions[indices[i] as usize * 3 + 2],
            );
            let pb = glam::Vec3::new(
                positions[indices[i + 1] as usize * 3],
                positions[indices[i + 1] as usize * 3 + 1],
                positions[indices[i + 1] as usize * 3 + 2],
            );
            let pc = glam::Vec3::new(
                positions[indices[i + 2] as usize * 3],
                positions[indices[i + 2] as usize * 3 + 1],
                positions[indices[i + 2] as usize * 3 + 2],
            );
            let na = glam::Vec3::new(
                normals[indices[i] as usize * 3],
                normals[indices[i] as usize * 3 + 1],
                normals[indices[i] as usize * 3 + 2],
            );
            let nb = glam::Vec3::new(
                normals[indices[i + 1] as usize * 3],
                normals[indices[i + 1] as usize * 3 + 1],
                normals[indices[i + 1] as usize * 3 + 2],
            );
            let nc = glam::Vec3::new(
                normals[indices[i + 2] as usize * 3],
                normals[indices[i + 2] as usize * 3 + 1],
                normals[indices[i + 2] as usize * 3 + 2],
            );
            self.push(Triangle {
                pa,
                pb,
                pc,
                na,
                nb,
                nc,
                material,
            });
        }
    }
}

struct SplitResult<'a> {
    left: Triangles<'a>,
    right: Triangles<'a>,
    cost: f32,
}

struct Triangles<'a> {
    triangle_list: &'a TriangleList,
    indices: Vec<usize>,
}
impl<'a> Triangles<'a> {
    fn aabb(&self) -> AABB {
        let mut aabb = AABB {
            min: glam::Vec3::new(std::f32::MAX, std::f32::MAX, std::f32::MAX),
            max: glam::Vec3::new(std::f32::MIN, std::f32::MIN, std::f32::MIN),
        };
        for i in self.indices.iter() {
            aabb = aabb.merge(&self.triangle_list[*i].aabb());
        }
        aabb
    }

    fn count(&self) -> usize {
        self.indices.len()
    }

    fn split(
        &self,
        comparator: impl Fn(&Triangle, &Triangle) -> Ordering,
        parent_surface_area: f32,
    ) -> SplitResult<'a> {
        let mut sorted_indices = self.indices.clone();
        sorted_indices.sort_by(|a, b| comparator(&self.triangle_list[*a], &self.triangle_list[*b]));

        let mut min_cost = std::f32::MAX;
        let mut min_cost_index = 0;
        for i in 1..self.indices.len() {
            let left = Triangles {
                triangle_list: self.triangle_list,
                indices: sorted_indices[..i].to_vec(),
            };
            let right = Triangles {
                triangle_list: self.triangle_list,
                indices: sorted_indices[i..].to_vec(),
            };
            let cost = BVH::COST_T
                + left.aabb().surface_area() / parent_surface_area
                    * BVH::COST_LEAF
                    * left.count() as f32
                + right.aabb().surface_area() / parent_surface_area
                    * BVH::COST_LEAF
                    * right.count() as f32;
            if cost < min_cost {
                min_cost = cost;
                min_cost_index = i;
            }
        }

        let left = Triangles {
            triangle_list: self.triangle_list,
            indices: sorted_indices[..min_cost_index].to_vec(),
        };
        let right = Triangles {
            triangle_list: self.triangle_list,
            indices: sorted_indices[min_cost_index..].to_vec(),
        };
        SplitResult {
            left,
            right,
            cost: min_cost,
        }
    }

    fn split_x(&self, parent_surface_area: f32) -> SplitResult<'a> {
        self.split(
            |a, b| a.center().x.partial_cmp(&b.center().x).unwrap(),
            parent_surface_area,
        )
    }

    fn split_y(&self, parent_surface_area: f32) -> SplitResult<'a> {
        self.split(
            |a, b| a.center().y.partial_cmp(&b.center().y).unwrap(),
            parent_surface_area,
        )
    }

    fn split_z(&self, parent_surface_area: f32) -> SplitResult<'a> {
        self.split(
            |a, b| a.center().z.partial_cmp(&b.center().z).unwrap(),
            parent_surface_area,
        )
    }

    fn traverse(&self, ray: &Ray) -> TriangleHit {
        let mut min_hit = TriangleHit::Miss;
        for i in self.indices.iter() {
            let hit = self.triangle_list[*i].intersect(ray, BVH::RAY_EPSILON, BVH::RAY_MAX_T);
            if let TriangleHit::Hit { t, .. } = hit {
                if let TriangleHit::Hit { t: min_t, .. } = min_hit {
                    if t < min_t {
                        min_hit = hit;
                    }
                } else {
                    min_hit = hit;
                }
            }
        }
        min_hit
    }
}

enum BVHNode<'a> {
    Leaf {
        triangles: Triangles<'a>,
        aabb: AABB,
    },
    Node {
        left: Box<BVHNode<'a>>,
        right: Box<BVHNode<'a>>,
        aabb: AABB,
    },
}
impl<'a> BVHNode<'a> {
    fn traverse(&self, ray: &Ray) -> TriangleHit {
        match self {
            BVHNode::Leaf { triangles, aabb } => {
                if let AABBHit::Hit { .. } = aabb.intersect(ray) {
                    triangles.traverse(ray)
                } else {
                    TriangleHit::Miss
                }
            }
            BVHNode::Node { left, right, aabb } => {
                if let AABBHit::Miss = aabb.intersect(ray) {
                    return TriangleHit::Miss;
                }

                let left_hit = left.traverse(ray);
                let right_hit = right.traverse(ray);
                match (left_hit, right_hit) {
                    (TriangleHit::Miss, TriangleHit::Miss) => TriangleHit::Miss,
                    (TriangleHit::Miss, hit) => hit,
                    (hit, TriangleHit::Miss) => hit,
                    (TriangleHit::Hit { t: t1, .. }, TriangleHit::Hit { t: t2, .. }) => {
                        if t1 < t2 {
                            left_hit
                        } else {
                            right_hit
                        }
                    }
                }
            }
        }
    }
}

struct BVH<'a> {
    root: BVHNode<'a>,
}
impl<'a> BVH<'a> {
    const COST_LEAF: f32 = 1.0;
    const COST_T: f32 = 1.0;

    const RAY_EPSILON: f32 = 0.001;
    const RAY_MAX_T: f32 = 1e12;

    fn build(triangle_list: &'a TriangleList) -> Self {
        let root = Self::build_node(Triangles {
            triangle_list,
            indices: (0..triangle_list.len()).collect(),
        });
        Self { root }
    }

    fn build_node(triangles: Triangles) -> BVHNode {
        if triangles.indices.len() == 1 {
            return BVHNode::Leaf {
                aabb: triangles.aabb(),
                triangles,
            };
        }

        let no_split_cost = BVH::COST_LEAF * triangles.count() as f32;
        let no_split_surface_area = triangles.aabb().surface_area();
        let split_x = triangles.split_x(no_split_surface_area);
        let split_y = triangles.split_y(no_split_surface_area);
        let split_z = triangles.split_z(no_split_surface_area);

        if no_split_cost <= split_x.cost
            && no_split_cost <= split_y.cost
            && no_split_cost <= split_z.cost
        {
            return BVHNode::Leaf {
                aabb: triangles.aabb(),
                triangles,
            };
        } else if split_x.cost <= split_y.cost && split_x.cost <= split_z.cost {
            return BVHNode::Node {
                left: Box::new(Self::build_node(split_x.left)),
                right: Box::new(Self::build_node(split_x.right)),
                aabb: triangles.aabb(),
            };
        } else if split_y.cost <= split_z.cost {
            return BVHNode::Node {
                left: Box::new(Self::build_node(split_y.left)),
                right: Box::new(Self::build_node(split_y.right)),
                aabb: triangles.aabb(),
            };
        } else {
            return BVHNode::Node {
                left: Box::new(Self::build_node(split_z.left)),
                right: Box::new(Self::build_node(split_z.right)),
                aabb: triangles.aabb(),
            };
        }
    }

    fn traverse(&self, ray: &Ray) -> TriangleHit {
        self.root.traverse(ray)
    }
}

struct Camera {
    up: glam::Vec3,
    view_dir: glam::Vec3,
    position: glam::Vec3,
    fov: f32,
}
impl Camera {
    fn new(up: glam::Vec3, view_dir: glam::Vec3, position: glam::Vec3, fov: f32) -> Self {
        Self {
            up,
            view_dir: view_dir.normalize(),
            position,
            fov,
        }
    }

    fn get_ray(
        &self,
        mut rng: &mut rand::rngs::ThreadRng,
        x: u32,
        y: u32,
        res_x: u32,
        res_y: u32,
    ) -> Ray {
        let aspect_ratio = res_x as f32 / res_y as f32;

        let uniform = Uniform::new(0.0, 2.0);
        let (rx, ry): (f32, f32) = (uniform.sample(&mut rng), uniform.sample(&mut rng));
        let rx = if rx < 1.0 {
            rx.sqrt() - 1.0
        } else {
            1.0 - (2.0 - rx).sqrt()
        };
        let ry = if ry < 1.0 {
            ry.sqrt() - 1.0
        } else {
            1.0 - (2.0 - ry).sqrt()
        };

        let fov = self.fov.to_radians();
        let tan_fov = (fov / 2.0).tan();
        let dir = glam::Vec3::new(
            (2.0 * (x as f32 + 0.5 + rx) / res_x as f32 - 1.0) * aspect_ratio * tan_fov,
            (1.0 - 2.0 * (y as f32 + 0.5 + ry) / res_y as f32) * tan_fov,
            -1.0,
        );

        let front = -self.view_dir;
        let right = self.up.cross(front).normalize();
        let up = front.cross(right).normalize();

        let dir = glam::Mat3::from_cols(right, up, front).mul_vec3(dir);

        Ray {
            origin: self.position,
            dir: dir.normalize(),
        }
    }
}

fn path_trace(mut rng: &mut ThreadRng, ray: &Ray, bvh: &BVH, depth: u32) -> glam::Vec3 {
    const MIN_DEPTH: u32 = 15;
    const MAX_DEPTH: u32 = 150;

    let hit = bvh.traverse(ray);

    match hit {
        TriangleHit::Miss => glam::Vec3::ZERO,
        TriangleHit::Hit {
            position,
            normal,
            material,
            ..
        } => {
            let uniform = Uniform::new(0.0, 1.0);
            let russian_roulette_probability = if depth <= MIN_DEPTH {
                1.0
            } else {
                material.russian_roulette_probability()
            };

            if depth > MAX_DEPTH {
                return glam::Vec3::ZERO;
            } else if depth > MIN_DEPTH {
                if uniform.sample(&mut rng) >= russian_roulette_probability {
                    return glam::Vec3::ZERO;
                }
            }

            let (normal, front_hit) = if normal.dot(ray.dir) <= 0.0 {
                (normal, true)
            } else {
                (-normal, false)
            };

            let up = if 1.0 - normal.dot(glam::Vec3::Y).abs() < 0.0001 {
                glam::Vec3::Z
            } else {
                glam::Vec3::Y
            };

            let tangent_x = normal.cross(up).normalize();
            let tangent_z = tangent_x.cross(normal).normalize();
            let tangent_to_world = glam::Mat3::from_cols(tangent_x, normal, tangent_z);
            let world_to_tangent = tangent_to_world.inverse();

            let view_dir_tangent = world_to_tangent.mul_vec3(-ray.dir).normalize();

            if let Some(sample) = material.sample(view_dir_tangent, front_hit, &mut rng) {
                let sample_dir_world = tangent_to_world
                    .mul_vec3(sample.sample_dir_tangent)
                    .normalize();

                let ray = Ray {
                    origin: position,
                    dir: sample_dir_world,
                };

                sample.bsdf_multiplied_cos_divided_by_pdf
                    * path_trace(&mut rng, &ray, bvh, depth + 1)
                    / (russian_roulette_probability)
                    + material.emissive()
            } else {
                material.emissive()
            }
        }
    }
}

fn main() {
    let mut triangle_list = TriangleList::new();

    let load_options = tobj::LoadOptions {
        single_index: true,
        triangulate: true,
        ignore_points: true,
        ignore_lines: true,
    };
    let models = [
        (
            "./assets/box.obj",
            Material::Lambert {
                color: glam::vec3(1.0, 1.0, 1.0),
            },
        ),
        (
            "./assets/bunny.obj",
            Material::Glass {
                inner_eta: 1.45,
                outer_eta: 1.0,
            },
        ),
        (
            "./assets/yuka.obj",
            Material::Lambert {
                color: glam::vec3(0.25, 0.25, 0.25),
            },
        ),
        (
            "./assets/migi.obj",
            Material::Lambert {
                color: glam::vec3(0.0, 0.25, 0.0),
            },
        ),
        (
            "./assets/hidari.obj",
            Material::Lambert {
                color: glam::vec3(0.25, 0.0, 0.0),
            },
        ),
        (
            "./assets/tenjou.obj",
            Material::Lambert {
                color: glam::vec3(0.25, 0.25, 0.25),
            },
        ),
        (
            "./assets/oku.obj",
            Material::Lambert {
                color: glam::vec3(0.25, 0.25, 0.25),
            },
        ),
        (
            "./assets/light.obj",
            Material::Emissive {
                color: glam::vec3(1.0, 1.0, 1.0),
                strength: 15.0,
            },
        ),
    ];

    for (path, material) in models {
        let (models, _) = tobj::load_obj(path, &load_options).expect("Failed to load OBJ file");
        for model in models {
            triangle_list.add_model(&model, material)
        }
    }

    println!("Start building BVH");
    let start = std::time::Instant::now();

    let bvh = BVH::build(&triangle_list);

    let end = start.elapsed();
    println!("Finished building BVH in {}s", end.as_secs_f32());

    let camera = Camera::new(
        glam::Vec3::Y,
        glam::vec3(0.0, -1.0, -3.0).normalize(),
        glam::Vec3::new(0.0, 3.5, 5.0),
        60.0,
    );

    let samples = 2_u32.pow(15);
    let width = 800;
    let height = 600;
    let l_white = 30.0_f32;

    let mut img = ImageBuffer::new(width, height);

    println!("Start rendering");
    let start = std::time::Instant::now();

    img.enumerate_pixels_mut()
        .collect::<Vec<(u32, u32, &mut Rgb<u8>)>>()
        .par_iter_mut()
        .for_each(|(x, y, pixel)| {
            let mut rng = rand::thread_rng();

            let mut rgb = glam::Vec3::ZERO;
            for _ in 0..samples {
                let ray = camera.get_ray(&mut rng, *x, *y, width, height);
                rgb += path_trace(&mut rng, &ray, &bvh, 0);
            }
            let rgb = rgb / samples as f32;

            // Reinhard
            let r = (rgb.x * (1.0 + rgb.x / l_white.powi(2))) / (1.0 + rgb.x);
            let g = (rgb.y * (1.0 + rgb.y / l_white.powi(2))) / (1.0 + rgb.y);
            let b = (rgb.z * (1.0 + rgb.z / l_white.powi(2))) / (1.0 + rgb.z);

            // gamma correction
            let r = r.powf(1.0 / 2.2);
            let g = g.powf(1.0 / 2.2);
            let b = b.powf(1.0 / 2.2);

            pixel[0] = (r * 255.0).min(255.0) as u8;
            pixel[1] = (g * 255.0).min(255.0) as u8;
            pixel[2] = (b * 255.0).min(255.0) as u8;
        });

    let end = start.elapsed();
    println!("Finished rendering in {}s", end.as_secs_f32());

    img.save("output.png").unwrap();
}
