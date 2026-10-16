use std::path::Path;

use glam::{Mat4, Vec3};
use gltf::{Document, buffer::Source, mesh::Mode};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::LoaderError;

pub struct Glb {
    doc: Document,
    buffers: Vec<Vec<u8>>,
}

pub struct Triangles {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[usize; 3]>,
}

fn model_error(e: impl std::fmt::Display) -> LoaderError {
    LoaderError::Model(e.to_string())
}

impl Glb {
    pub fn load(path: &Path) -> Result<Self, LoaderError> {
        let gltf::Gltf { document, blob } = gltf::Gltf::from_slice(&std::fs::read(path)?).map_err(model_error)?;

        let mut blob = blob;

        let buffers = document
            .buffers()
            .map(|b| match b.source() {
                Source::Bin => blob.take().ok_or_else(|| LoaderError::Model("its binary chunk is missing".into())),
                Source::Uri(uri) => Ok(std::fs::read(path.with_file_name(uri))?),
            })
            .collect::<Result<_, _>>()?;

        Ok(Self { doc: document, buffers })
    }

    fn node(&self, node: usize) -> Option<gltf::Node<'_>> {
        self.doc.nodes().nth(node)
    }

    pub fn root(&self) -> Option<usize> {
        let scene = self.doc.default_scene().or_else(|| self.doc.scenes().next())?;

        scene.nodes().next().map(|n| n.index())
    }

    pub fn children(&self, node: usize) -> Vec<usize> {
        self.node(node).map_or(Vec::new(), |n| n.children().map(|c| c.index()).collect())
    }

    pub fn name(&self, node: usize) -> &str {
        self.node(node).and_then(|n| n.name()).unwrap_or("")
    }

    pub fn property<T: DeserializeOwned>(&self, node: usize, key: &str) -> Result<Option<T>, LoaderError> {
        let Some(raw) = self.node(node).and_then(|n| n.extras().as_ref().map(|r| r.get().to_string())) else { return Ok(None) };
        let extras: Value = serde_json::from_str(&raw)?;

        extras.get(key).map(|v| serde_json::from_value(v.clone())).transpose().map_err(LoaderError::from)
    }

    pub fn translation(&self, node: usize) -> Vec3 {
        self.node(node).map_or(Vec3::ZERO, |n| Vec3::from_array(n.transform().decomposed().0))
    }

    pub fn scale(&self, node: usize) -> Vec3 {
        self.node(node).map_or(Vec3::ONE, |n| Vec3::from_array(n.transform().decomposed().2))
    }

    /// A mesh's triangle primitives (those whose material `keep` takes, by name) appended to `out`, placed by `m`.
    fn add_mesh(&self, mesh: gltf::Mesh<'_>, m: Mat4, keep: &dyn Fn(&str) -> bool, out: &mut Triangles) -> Result<(), LoaderError> {
        for p in mesh.primitives().filter(|p| keep(p.material().name().unwrap_or(""))) {
            if p.mode() != Mode::Triangles {
                return Err(LoaderError::Model("its mesh is not all triangles".into()));
            }

            let reader = p.reader(|b| self.buffers.get(b.index()).map(Vec::as_slice));
            let base = out.positions.len();
            let positions: Vec<Vec3> = reader.read_positions().ok_or_else(|| LoaderError::Model("a mesh has no positions".into()))?.map(Vec3::from_array).collect();
            let indices: Vec<usize> = match reader.read_indices() {
                Some(i) => i.into_u32().map(|i| i as usize).collect(),
                None => (0..positions.len()).collect(),
            };

            if !indices.len().is_multiple_of(3) || indices.iter().any(|&i| i >= positions.len()) {
                return Err(LoaderError::Model("its mesh is not all triangles".into()));
            }

            out.triangles.extend(indices.as_chunks::<3>().0.iter().map(|t| [base + t[0], base + t[1], base + t[2]]));
            out.positions.extend(positions.into_iter().map(|p| if m == Mat4::IDENTITY { p } else { m.transform_point3(p) }));
        }

        Ok(())
    }

    /// The triangles of a node's mesh (every primitive's, one after another), placed by the node's own transform;
    /// None for a node without a mesh, an error for one with anything but triangle lists.
    pub fn triangles(&self, node: usize) -> Result<Option<Triangles>, LoaderError> {
        let Some(n) = self.node(node) else { return Ok(None) };
        let Some(mesh) = n.mesh() else { return Ok(None) };
        let mut out = Triangles { positions: Vec::new(), triangles: Vec::new() };

        self.add_mesh(mesh, Mat4::from_cols_array_2d(&n.transform().matrix()), &|_| true, &mut out)?;
        Ok(Some(out))
    }

    /// The triangles of every mesh at and under `root`, in the root's space: the root's own (its transform left
    /// out) and its descendants', but for the nodes `skip_node` takes by name (and theirs below them) and the
    /// primitives whose material `skip_material` takes by name.
    pub fn triangles_under(&self, root: usize, skip_node: &dyn Fn(&str) -> bool, skip_material: &dyn Fn(&str) -> bool) -> Result<Triangles, LoaderError> {
        let mut out = Triangles { positions: Vec::new(), triangles: Vec::new() };
        let mut stack = vec![(root, Mat4::IDENTITY)];

        while let Some((i, m)) = stack.pop() {
            let Some(n) = self.node(i) else { continue };

            if let Some(mesh) = n.mesh() {
                self.add_mesh(mesh, m, &|name| !skip_material(name), &mut out)?;
            }

            for c in n.children().filter(|c| !skip_node(c.name().unwrap_or(""))).collect::<Vec<_>>().into_iter().rev() {
                stack.push((c.index(), m * Mat4::from_cols_array_2d(&c.transform().matrix())));
            }
        }

        Ok(out)
    }
}

pub mod any_vec3 {
    use glam::Vec3;
    use serde::{Deserialize, Deserializer, Serializer, de::Error, ser::SerializeTuple};

    pub fn serialize<S: Serializer>(v: &Vec3, s: S) -> Result<S::Ok, S::Error> {
        let mut t = s.serialize_tuple(3)?;

        for c in v.to_array() {
            if c.is_finite() {
                t.serialize_element(&c)?;
            } else {
                t.serialize_element(&format!("{:#010x}", c.to_bits()))?;
            }
        }

        t.end()
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Component {
        Number(f32),
        Bits(String),
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec3, D::Error> {
        let parts: [Component; 3] = Deserialize::deserialize(d)?;
        let mut out = [0.0f32; 3];

        for (o, c) in out.iter_mut().zip(parts) {
            *o = match c {
                Component::Number(n) => n,
                Component::Bits(b) => f32::from_bits(u32::from_str_radix(b.trim_start_matches("0x"), 16).map_err(D::Error::custom)?),
            };
        }

        Ok(Vec3::from_array(out))
    }

    #[derive(Deserialize)]
    pub struct AnyVec3(#[serde(deserialize_with = "deserialize")] pub Vec3);
}
