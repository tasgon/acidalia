use crate::wgpu;
use acidalia_core::Nametag;
use acidalia_proc_macros::Nametag;
use dashmap::DashMap;
use notify::{Watcher, RecommendedWatcher};
use shaderc;
use std::{collections::hash_map::RandomState, ops::Deref, path::Path, sync::{Arc, RwLock, Weak}};
use std::{collections::HashMap, path::PathBuf};
use wgpu::{ShaderModule, ShaderModuleDescriptor};

use crate::graphics::GraphicsState;

type Manufacturer = Box<dyn Fn(&GraphicsState, RenderSet) -> wgpu::RenderPipeline + Send + Sync>;

struct ManufacturingData {
    manufacturer: Manufacturer,
    tags: RenderTags,
    pipeline: Weak<wgpu::RenderPipeline>,
}

impl ManufacturingData {
    pub fn new(manufacturer: Manufacturer, tags: RenderTags, pipeline: Weak<wgpu::RenderPipeline>) -> Self {
        Self {
            manufacturer,
            tags,
            pipeline,
        }
    }
}

pub struct ShaderSourceDescriptor {
    path: PathBuf,
    entry_point: String,
}

impl ShaderSourceDescriptor {
    pub fn filename(&self) -> &str {
        self.path
            .file_name()
            .map(|f| f.to_str().expect("Invalid file name"))
            .unwrap()
    }
}

type SMapRef<'a> = dashmap::mapref::one::Ref<'a, u128, (Option<ShaderSourceDescriptor>, ShaderModule), RandomState>;

pub struct ShaderRef<'a>(SMapRef<'a>);

impl<'a> Deref for ShaderRef<'a> {
    type Target = ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.0.1
    }
}

impl<'a> From<SMapRef<'a>> for ShaderRef<'a> {
    fn from(val: SMapRef<'a>) -> Self {
        Self(val)
    }
}

/// A struct that provides shader compilation and access from the program.
/// Utilizes `shaderc` to compile GLSL source into SPIR-V.
pub struct ShaderState {
    compiler: shaderc::Compiler,
    shader_map: DashMap<u128, (Option<ShaderSourceDescriptor>, ShaderModule)>,
    manufacturers: Arc<RwLock<Vec<ManufacturingData>>>,
    watcher: RecommendedWatcher,
}

impl ShaderState {
    /// Construct a new `ShaderState`
    pub fn new() -> Self {
        let compiler = shaderc::Compiler::new().unwrap();
        let manufacturers = Arc::new(RwLock::new(vec![]));
        let mfptr = Arc::clone(&manufacturers);
        let shader_map = DashMap::new();
        let watcher: RecommendedWatcher = Watcher::new_immediate(move |ev| match ev {
            Ok(event) => {
//                 for entry in shader_map.iter_mut() {
//                     // let v: Option<ShaderSourceDescriptor> = entry.value_mut().0;
//                     // if let Some(desc) = entry.value_mut().0 {
// // 
//                     // }
//                 }
//                 let mut mfs = mfptr.write().unwrap();
//                 mfs.retain(|i: &ManufacturingData| {
//                     i.pipeline.clone().upgrade().is_some()
//                 });
//                 for data in mfs.iter() {
                    
//                 }
            }
            Err(e) => println!("Watch error: {:?}", e),
        }).unwrap();
        Self {
            compiler,
            shader_map,
            manufacturers,
            watcher,
        }
    }

    /// Loads a shader from a file. In the future, shaders reloaded from here
    /// will have hot-reloading.
    pub fn load_src(
        &mut self,
        key: impl Nametag,
        path: impl AsRef<Path>,
        entry_point: impl Into<String>,
        kind: shaderc::ShaderKind,
        options: Option<&shaderc::CompileOptions>,
        gs: &mut GraphicsState,
    ) {
        let path = path.as_ref().to_owned();
        let entry_point: String = entry_point.into();
        let src_desc = ShaderSourceDescriptor { path, entry_point };
        let data = std::fs::read_to_string(&src_desc.path)
            .expect(&format!("Unable to read from {}!", src_desc.filename()));
        let res = self
            .compiler
            .compile_into_spirv(
                &data,
                kind,
                src_desc.filename(),
                &src_desc.entry_point,
                options,
            )
            .unwrap();
        self.watcher.watch(&src_desc.path, notify::RecursiveMode::NonRecursive).unwrap();
        let desc = ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::SpirV(res.as_binary().into()),
            flags: wgpu::ShaderFlags::default(),
        };
        self.shader_map.insert(
            key.tag(),
            (Some(src_desc), gs.device.create_shader_module(&desc)),
        );
    }

    /// Loads a shader from an `&str` source string.
    pub fn load_str(
        &mut self,
        key: impl Nametag,
        filename: &str,
        src: &str,
        entry_point: &str,
        kind: shaderc::ShaderKind,
        options: Option<&shaderc::CompileOptions>,
        gs: &mut GraphicsState,
    ) {
        let res = self
            .compiler
            .compile_into_spirv(src, kind, filename, entry_point, options)
            .unwrap();
        let desc = ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::SpirV(res.as_binary().into()),
            flags: wgpu::ShaderFlags::default(),
        };
        self.shader_map
            .insert(key.tag(), (None, gs.device.create_shader_module(&desc)));
    }

    /// Attempt to retrieve a shader with a given tag `key`.
    pub fn get(&self, key: impl Nametag) -> Option<ShaderRef> {
        self.shader_map.get(&key.tag()).map(|i| i.into())
    }

    pub(crate) fn init_shaders(&mut self, gs: &mut GraphicsState) {
        self.load_str(
            InternalShaders::IcedVert,
            "iced.vert",
            include_str!("gl/iced.vert"),
            "main",
            shaderc::ShaderKind::Vertex,
            None,
            gs,
        );
        self.load_str(
            InternalShaders::IcedFrag,
            "iced.frag",
            include_str!("gl/iced.frag"),
            "main",
            shaderc::ShaderKind::Fragment,
            None,
            gs,
        );
    }

    fn create_render_set(&self, tags: RenderTags) -> RenderSet {
        let vertex = self.get(tags.vertex).expect("No shader registered with vertex tag");
        let fragment = self.get(tags.fragment).expect("No shader registered with fragment tag");
        RenderSet {
            vertex,
            fragment,
        }
    }

    pub fn pipeline(
        &mut self,
        gs: &GraphicsState,
        f: impl Fn(&GraphicsState, RenderSet) -> wgpu::RenderPipeline + Send + Sync + 'static,
        tags: RenderTags,
    ) -> Arc<wgpu::RenderPipeline> {
        let manufacturer = Box::new(f) as Manufacturer;
        let ret = Arc::new((manufacturer)(gs, self.create_render_set(tags)));
        let val = ManufacturingData::new(manufacturer, tags, Arc::downgrade(&ret));
        self.manufacturers.write().unwrap().push(val);
        ret
    }
}

/// The key enums for the internal shaders.
#[derive(Nametag)]
pub enum InternalShaders {
    IcedVert,
    IcedFrag,
}

#[derive(Hash, Copy, Clone)]
pub struct RenderTags {
    pub vertex: u128,
    pub fragment: u128,
}

impl RenderTags {
    pub fn new(vertex: impl Nametag, fragment: impl Nametag) -> Self {
        Self {
            vertex: vertex.tag(),
            fragment: fragment.tag(),
        }
    }
}

impl RenderTags {
    fn has_tag(&self, tag: u128) -> bool {
        self.vertex == tag || self.fragment == tag
    }
}

pub struct RenderSet<'a> {
    pub vertex: ShaderRef<'a>,
    pub fragment: ShaderRef<'a>,
}
