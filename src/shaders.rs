use crate::wgpu;
use acidalia_core::Nametag;
use acidalia_proc_macros::Nametag;
use crossbeam_channel::Sender;
use dashmap::DashMap;
use notify::{EventKind, RecommendedWatcher, Watcher, event::{AccessKind, AccessMode}};
use shaderc;
use std::path::PathBuf;
use std::{
    collections::hash_map::RandomState,
    ops::Deref,
    path::Path,
    sync::{Arc, RwLock, Weak},
    thread::JoinHandle,
};
use wgpu::{ShaderModule, ShaderModuleDescriptor};

use crate::graphics::GraphicsState;

type Manufacturer = Box<dyn Fn(&wgpu::Device, RenderSet) -> wgpu::RenderPipeline + Send + Sync>;

struct ManufacturingData {
    manufacturer: Manufacturer,
    tags: RenderTags,
    pipeline: Weak<wgpu::RenderPipeline>,
}

impl ManufacturingData {
    pub fn new(
        manufacturer: Manufacturer,
        tags: RenderTags,
        pipeline: Weak<wgpu::RenderPipeline>,
    ) -> Self {
        Self {
            manufacturer,
            tags,
            pipeline,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShaderSourceDescriptor {
    path: Option<PathBuf>,
    filename: Option<String>,
    data: Option<String>,
    entry_point: String,
    kind: shaderc::ShaderKind,
}

impl ShaderSourceDescriptor {
    pub fn filename(&self) -> String {
        match self.filename.as_ref() {
            Some(f) => f.clone(),
            None => (&self.path.as_ref())
            .unwrap()
            .file_name()
            .map(|f| f.to_str().expect("Invalid file name"))
            .unwrap()
            .to_owned(),
        }
    }
}

type SMapRef<'a> = dashmap::mapref::one::Ref<
    'a,
    u128,
    (Option<ShaderSourceDescriptor>, ShaderModule),
    RandomState,
>;

pub struct ShaderRef<'a>(SMapRef<'a>);

impl<'a> Deref for ShaderRef<'a> {
    type Target = ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.0 .1
    }
}

impl<'a> From<SMapRef<'a>> for ShaderRef<'a> {
    fn from(val: SMapRef<'a>) -> Self {
        Self(val)
    }
}

#[derive(Debug)]
enum CompilerMessage {
    FromFile(u128, ShaderSourceDescriptor),
    FromString(u128, ShaderSourceDescriptor),
}

type ShaderMap = DashMap<u128, (Option<ShaderSourceDescriptor>, ShaderModule)>;

fn get_shader_ref(map: &ShaderMap, key: impl Nametag) -> Option<ShaderRef> {
    map.get(&key.tag()).map(|i| i.into())
}

fn create_render_set(map: &ShaderMap, tags: RenderTags) -> RenderSet {
    let vertex = get_shader_ref(map, tags.vertex)
        .expect("No shader registered with vertex tag");
    let fragment = get_shader_ref(map, tags.fragment)
        .expect("No shader registered with fragment tag");
    RenderSet { vertex, fragment }
}

/// A struct that provides shader compilation and access from the program.
/// Utilizes `shaderc` to compile GLSL source into SPIR-V.
pub struct ShaderState {
    //compiler: Arc<Mutex<shaderc::Compiler>>,
    shader_map: Arc<ShaderMap>,
    manufacturers: Arc<RwLock<Vec<ManufacturingData>>>,
    watcher: RecommendedWatcher,
    _handle: JoinHandle<()>,
    tx: Sender<CompilerMessage>,
}

impl ShaderState {
    /// Construct a new `ShaderState`.
    pub fn new(gs: &GraphicsState) -> Self {
        let manufacturers = Arc::new(RwLock::new(vec![]));
        let mfptr = Arc::clone(&manufacturers);
        let map: ShaderMap = ShaderMap::new();
        let shader_map = Arc::new(map);
        let sm = Arc::clone(&shader_map);
        let (tx, rx) = crossbeam_channel::unbounded::<CompilerMessage>();
        let tx2 = tx.clone();
        let watcher: RecommendedWatcher =
            Watcher::new_immediate(move |ev: Result<notify::Event, notify::Error>| match ev {
                Ok(event) => {
                    println!("{:?}", event);
                    if event.kind != EventKind::Access(AccessKind::Close(AccessMode::Write)) { return; } 
                    for mut entry in sm.iter_mut() {
                        let key = *entry.key();
                        if let Some(src_desc) = &entry.value_mut().0 {
                            if let Some(path) = &src_desc.path {
                                let path = std::fs::canonicalize(path).unwrap();
                                if event.paths.iter().map(|f| std::fs::canonicalize(f).unwrap()).collect::<Vec<_>>().contains(&path) {
                                    tx2.send(CompilerMessage::FromFile(key, src_desc.clone())).unwrap();
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("Watch error: {:?}", e),
            })
            .unwrap();
        let sm = Arc::clone(&shader_map);
        let device = Arc::clone(&gs.device);
        let _handle = std::thread::spawn(move || {
            let mut compiler = shaderc::Compiler::new().unwrap();
            'yeet: loop {
                let val = rx.recv();
                match val {
                    Ok(msg) => {
                        let key: u128;
                        let source_descriptor: Option<ShaderSourceDescriptor>;
                        let res: Result<shaderc::CompilationArtifact, shaderc::Error>;
                        let filename: String;
                        match msg {
                            CompilerMessage::FromFile(key_, src_desc) => {
                                key = key_;
                                filename = src_desc.filename();
                                let data = std::fs::read_to_string(&src_desc.path.as_ref().unwrap())
                                    .expect(&format!("Unable to read from {}!", filename));
                                res = compiler
                                    .compile_into_spirv(
                                        &data,
                                        src_desc.kind,
                                        &src_desc.filename(),
                                        &src_desc.entry_point,
                                        None,
                                    );
                                source_descriptor = Some(src_desc);
                            }
                            CompilerMessage::FromString(key_, src_desc) => {
                                key = key_;
                                filename = src_desc.filename();
                                res = compiler
                                    .compile_into_spirv(
                                        &src_desc.data.as_ref().unwrap(),
                                        src_desc.kind,
                                        &src_desc.filename(),
                                        &src_desc.entry_point,
                                        None,
                                    );
                                source_descriptor = None;
                            }

                        }
                        match res {
                            Ok(res) => {
                                let desc = ShaderModuleDescriptor {
                                    label: None,
                                    source: wgpu::ShaderSource::SpirV(res.as_binary().into()),
                                    flags: wgpu::ShaderFlags::default(),
                                };
                                sm.insert(key, (source_descriptor, device.create_shader_module(&desc)));
                                println!("Compiled {}", filename);
                            }
                            Err(e) => {
                                eprintln!("Failed to recompile '{}': {}", filename, e);
                                continue;
                            }
                        }
                        let mut mfs = mfptr.write().unwrap();
                        mfs.retain(|i: &ManufacturingData| i.pipeline.clone().upgrade().is_some());
                        for data in mfs.iter() {
                            if data.tags.has_tag(key) {
                                let render_set = create_render_set(&sm, data.tags);
                                let new_pipeline = (data.manufacturer)(&device, render_set);

                                if let Some(pipe_ref) = data.pipeline.upgrade() {
                                    unsafe { *(Arc::<wgpu::RenderPipeline>::as_ptr(&pipe_ref) as *mut wgpu::RenderPipeline) = new_pipeline; }
                                }
                            }
                        }
                    },
                    Err(_) => { break 'yeet; }
                }
            }
        });

        //println!("{}", std::env::current_dir().unwrap().to_str().unwrap());

        Self {
            shader_map,
            manufacturers,
            watcher,
            _handle,
            tx,
        }
    }

    /// Loads a shader from a file. In the future, shaders reloaded from here
    /// will have hot-reloading.
    pub fn load_file(
        &mut self,
        key: impl Nametag,
        path: impl AsRef<Path>,
        entry_point: impl Into<String>,
        kind: shaderc::ShaderKind,
        _options: Option<&shaderc::CompileOptions>,
    ) {
        let path = Some(path.as_ref().to_owned());
        let entry_point: String = entry_point.into();
        let src_desc = ShaderSourceDescriptor {
            path,
            entry_point,
            kind,
            filename: None,
            data: None,
        };
        self.watcher
            .watch(
                &src_desc.path.as_ref().unwrap(),
                notify::RecursiveMode::NonRecursive,
            )
            .unwrap();

        let tag = key.tag();
        self.tx
            .send(CompilerMessage::FromFile(tag, src_desc))
            .unwrap();

        while !self.shader_map.contains_key(&tag) {
            continue;
        }
    }

    /// Loads a shader from an `&str` source string.
    pub fn load_src(
        &mut self,
        key: impl Nametag,
        filename: &str,
        src: &str,
        entry_point: &str,
        kind: shaderc::ShaderKind,
        _options: Option<&shaderc::CompileOptions>,
    ) {
        let src_desc = ShaderSourceDescriptor {
            path: None,
            filename: Some(filename.to_owned()),
            data: Some(src.to_owned()),
            entry_point: entry_point.to_owned(),
            kind,
        };
        let tag = key.tag();
        self.tx
            .send(CompilerMessage::FromString(tag, src_desc))
            .unwrap();
        
        while !self.shader_map.contains_key(&tag) {
            continue;
        }
    }

    /// Attempt to retrieve a shader with a given tag `key`.
    pub fn get(&self, key: impl Nametag) -> Option<ShaderRef> {
        self.shader_map.get(&key.tag()).map(|i| i.into())
    }

    pub(crate) fn init_shaders(&mut self) {
        self.load_src(
            InternalShaders::IcedVert,
            "iced.vert",
            include_str!("gl/iced.vert"),
            "main",
            shaderc::ShaderKind::Vertex,
            None,
        );
        self.load_src(
            InternalShaders::IcedFrag,
            "iced.frag",
            include_str!("gl/iced.frag"),
            "main",
            shaderc::ShaderKind::Fragment,
            None,
        );

        // self.load_file(
        //     InternalShaders::IcedVert,
        //     "../acidalia/src/gl/iced.vert",
        //     "main",
        //     shaderc::ShaderKind::Vertex,
        //     None,
        // );
        // self.load_file(
        //     InternalShaders::IcedFrag,
        //     "../acidalia/src/gl/iced.frag",
        //     "main",
        //     shaderc::ShaderKind::Fragment,
        //     None,
        // );
    }

    fn create_render_set(&self, tags: RenderTags) -> RenderSet {
        let vertex = self
            .get(tags.vertex)
            .expect("No shader registered with vertex tag");
        let fragment = self
            .get(tags.fragment)
            .expect("No shader registered with fragment tag");
        RenderSet { vertex, fragment }
    }

    pub fn pipeline(
        &mut self,
        gs: &GraphicsState,
        f: impl Fn(&wgpu::Device, RenderSet) -> wgpu::RenderPipeline + Send + Sync + 'static,
        tags: RenderTags,
    ) -> Arc<wgpu::RenderPipeline> {
        let manufacturer = Box::new(f) as Manufacturer;
        let ret = Arc::new((manufacturer)(&gs.device, self.create_render_set(tags)));
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
