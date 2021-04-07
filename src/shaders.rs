// The single worst mistake that I have made so far was to try and be clever about hot reloading.
// The second worst mistake was doing those """clever""" tricks specific to my shader management code.
// To conclude, beware he who has to go through this file.
// TODO: move the manufactury into its own crate probably and generally be way smarter about this

use crate::{wgpu};
use acidalia_core::Nametag;
use acidalia_proc_macros::Nametag;
use crossbeam_channel::Sender;
use dashmap::DashMap;
use notify::{
    event::{AccessKind, AccessMode},
    EventKind, RecommendedWatcher, Watcher,
};
use shaderc;
use std::{path::PathBuf};
use std::{
    collections::hash_map::RandomState,
    ops::Deref,
    path::Path,
    sync::{Arc, RwLock, Weak},
    thread::JoinHandle,
};
use wgpu::{ComputePipeline, PipelineLayout, RenderPipeline, ShaderModule, ShaderModuleDescriptor};

use crate::graphics::GraphicsState;

#[derive(derive_more::From)]
enum ManufacturingOutput {
    RenderPipeline(wgpu::RenderPipeline),
    ComputePipeline(wgpu::ComputePipeline),
}

impl ManufacturingOutput {
    fn render(self) -> wgpu::RenderPipeline {
        if let ManufacturingOutput::RenderPipeline(val) = self {
            return val;
        }
        panic!("This isn't a render pipeline")
    }

    fn compute(self) -> wgpu::ComputePipeline {
        if let ManufacturingOutput::ComputePipeline(val) = self {
            return val;
        }
        panic!("This isn't a compute pipeline")
    }

    unsafe fn swap(&self, val: &Arc<dyn Send + Sync>) {
        match self {
            ManufacturingOutput::RenderPipeline(v) => {
                let ptr = (v as *const _) as *mut RenderPipeline;
                std::ptr::swap(Arc::as_ptr(val) as *mut RenderPipeline, ptr)
            }
            ManufacturingOutput::ComputePipeline(v) => {
                let ptr = (v as *const _) as *mut ComputePipeline;
                std::ptr::swap(Arc::as_ptr(val) as *mut ComputePipeline, ptr)
            }
        }
    }
}

type Manufacturer = Box<dyn Fn(&wgpu::Device, ShaderSet) -> ManufacturingOutput + Send + Sync>;

struct ManufacturingData {
    manufacturer: Manufacturer,
    tags: ShaderTags,
    pipeline: Weak<dyn Send + Sync>,
}

impl ManufacturingData {
    fn new(manufacturer: Manufacturer, tags: ShaderTags, pipeline: Weak<dyn Send + Sync>) -> Self {
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
    CullRefs,
    #[allow(dead_code)]
    Interrupt, // TODO: think about if i really need this
}

type ShaderMap = DashMap<u128, (Option<ShaderSourceDescriptor>, ShaderModule)>;

fn get_shader_ref(map: &ShaderMap, key: impl Nametag) -> Option<ShaderRef> {
    map.get(&key.tag()).map(|i| i.into())
}

const LABELS: &'static [&'static str] = &["vertex", "fragment", "compute"];

fn create_render_set(map: &ShaderMap, tags: ShaderTags) -> ShaderSet {
    let tags = [tags.vertex, tags.fragment, tags.compute];
    let mut map = tags
        .iter()
        .zip(LABELS.iter())
        .map(|(t, l)| {
            t.map(|i| {
                get_shader_ref(map, i).expect(&format!("No shader registered with {} tag", l))
            })
        });
    ShaderSet {
        vertex: map.next().unwrap(),
        fragment: map.next().unwrap(),
        compute: map.next().unwrap(),
    }
}

/// A struct that provides shader compilation and access from the program.
/// Utilizes `shaderc` to compile GLSL source into SPIR-V.
pub struct ShaderState {
    shader_map: Arc<ShaderMap>,
    manufacturers: Arc<RwLock<Vec<ManufacturingData>>>,
    watcher: RecommendedWatcher,
    _handle: JoinHandle<()>,
    tx: Sender<CompilerMessage>,
    device: Arc<wgpu::Device>,
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
                    // TODO: log
                    // println!("{:?}", event);
                    if event.kind != EventKind::Access(AccessKind::Close(AccessMode::Write)) {
                        return;
                    }
                    for mut entry in sm.iter_mut() {
                        let key = *entry.key();
                        if let Some(src_desc) = &entry.value_mut().0 {
                            if let Some(path) = &src_desc.path {
                                let path = std::fs::canonicalize(path).unwrap();
                                if event
                                    .paths
                                    .iter()
                                    .map(|f| std::fs::canonicalize(f).unwrap())
                                    .collect::<Vec<_>>()
                                    .contains(&path)
                                {
                                    tx2.send(CompilerMessage::FromFile(key, src_desc.clone()))
                                        .unwrap();
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("Watch error: {:?}", e),
            })
            .unwrap();
        let sm = Arc::clone(&shader_map);
        // TODO: remove the ManuallyDrop when gfx-rs/wgpu-rs#837 gets dealt with
        let device = std::mem::ManuallyDrop::new(Arc::clone(&gs.device));
        let _handle = std::thread::spawn(move || {
            let mut compiler = shaderc::Compiler::new().unwrap();
            let mut garbage = vec![];
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
                                let data =
                                    std::fs::read_to_string(&src_desc.path.as_ref().unwrap())
                                        .expect(&format!("Unable to read from {}!", filename));
                                res = compiler.compile_into_spirv(
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
                                res = compiler.compile_into_spirv(
                                    &src_desc.data.as_ref().unwrap(),
                                    src_desc.kind,
                                    &src_desc.filename(),
                                    &src_desc.entry_point,
                                    None,
                                );
                                source_descriptor = Some(src_desc);
                            }
                            CompilerMessage::CullRefs => {
                                garbage.clear();
                                continue 'yeet;
                            }
                            CompilerMessage::Interrupt => {
                                break 'yeet;
                            }
                        }
                        match res {
                            Ok(res) => {
                                let desc = ShaderModuleDescriptor {
                                    label: None,
                                    source: wgpu::ShaderSource::SpirV(res.as_binary().into()),
                                    flags: wgpu::ShaderFlags::default(),
                                };
                                sm.insert(
                                    key,
                                    (source_descriptor, device.create_shader_module(&desc)),
                                );
                                // TODO: log
                                // println!("Compiled {}", filename);
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

                                if let Some(pipe_ref) = data.pipeline.clone().upgrade() {
                                    // TODO: this is probably way too much premature optimization, and I should rethink my
                                    // design soon. maybe just switch to an ecs or something similar?
                                    unsafe {
                                        new_pipeline.swap(&pipe_ref);
                                    }
                                    garbage.push(new_pipeline);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        break 'yeet;
                    }
                }
            }
        });

        Self {
            shader_map,
            manufacturers,
            watcher,
            _handle,
            tx,
            device: Arc::clone(&gs.device),
        }
    }

    /// Loads a shader from a file. Shaders added from here will hot-reload.
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

    /// Initialize the internal shaders for the program.
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
    }
    /// Start constructing a new pipeline using the [`RenderPipelineBuilder`].
    pub fn render_pipeline_builder<T: Into<String>>(
        &self,
        label: impl Into<Option<T>>,
        layout: wgpu::PipelineLayout,
        vertex: impl Nametag,
    ) -> RenderPipelineBuilder {
        RenderPipelineBuilder::new(self, label.into().map(|i| i.into()), layout, vertex)
    }

    /// Create a new compute pipeline.
    pub fn compute_pipeline<T: Into<String>>(&self, label: impl Into<Option<T>>, layout: impl Into<Option<wgpu::PipelineLayout>>, shader: impl Nametag) -> Arc<ComputePipeline> {
        ComputePipelineBuilder {
            state: self,
            label: label.into().map(|i| i.into()),
            layout: layout.into(),
            module: shader.tag(),
        }.build()
    }

    /// Tell the manufactory to cull all unused render pipelines.
    #[inline(always)]
    pub(crate) fn cull(&mut self) {
        self.tx.send(CompilerMessage::CullRefs).unwrap();
    }
}

/// The key enums for the internal shaders.
#[derive(Nametag)]
pub enum InternalShaders {
    IcedVert,
    IcedFrag,
}

#[derive(Hash, Copy, Clone)]
pub struct ShaderTags {
    pub vertex: Option<u128>,
    pub fragment: Option<u128>,
    pub compute: Option<u128>,
}

impl ShaderTags {
    fn render<T: Nametag>(vertex: impl Nametag, fragment: Option<T>) -> Self {
        Self {
            vertex: Some(vertex.tag()),
            fragment: fragment.map(|i| i.tag()),
            compute: None,
        }
    }

    fn compute(compute: impl Nametag) -> Self {
        Self {
            vertex: None,
            fragment: None,
            compute: Some(compute.tag())
        }
    }
}

impl ShaderTags {
    fn has_tag(&self, tag: u128) -> bool {
        [self.vertex, self.fragment]
            .iter()
            .map(|i| i.map(|j| j == tag).unwrap_or(false))
            .fold(false, |a, i| a | i)
    }
}

pub struct ShaderSet<'a> {
    pub vertex: Option<ShaderRef<'a>>,
    pub fragment: Option<ShaderRef<'a>>,
    pub compute: Option<ShaderRef<'a>>,
}

/// This tells the engine how to build your render pipelines.
/// Created from [`ShaderState::render_pipeline_builder`].
pub struct RenderPipelineBuilder<'a> {
    state: &'a ShaderState,
    label: Option<String>,
    layout: wgpu::PipelineLayout,
    vertex: u128,
    fragment: Option<(u128, Vec<wgpu::ColorTargetState>)>,
    primitive: wgpu::PrimitiveState,
    depth_stencil: Option<wgpu::DepthStencilState>,
    multisample: wgpu::MultisampleState,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub(crate) fn new(
        state: &'a ShaderState,
        label: Option<String>,
        layout: wgpu::PipelineLayout,
        vertex: impl Nametag,
    ) -> Self {
        Self {
            state,
            label,
            layout,
            vertex: vertex.tag(),
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        }
    }

    /// Set which fragment shader to use, and what [`wgpu::ColorTargetState`]s to target.
    pub fn fragment(
        mut self,
        fragment: impl Nametag,
        targets: impl ToVec<wgpu::ColorTargetState>,
    ) -> Self {
        self.fragment = Some((fragment.tag(), targets.to_vec()));
        self
    }

    /// Set the primitive state.
    pub fn primitive(mut self, primitive: wgpu::PrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }

    /// Set the depth stencil state.
    pub fn depth_stencil(
        mut self,
        depth_stencil: impl Into<Option<wgpu::DepthStencilState>>,
    ) -> Self {
        self.depth_stencil = depth_stencil.into();
        self
    }

    /// Set the [`wgpu::MultisampleState`] to use.
    pub fn multisample(mut self, count: u32, mask: u64, alpha_to_coverage_enabled: bool) -> Self {
        self.multisample = wgpu::MultisampleState {
            count,
            mask,
            alpha_to_coverage_enabled,
        };
        self
    }

    /// Add the info to the render pipeline manufactory and immediately give back a [`wgpu::RenderPipeline`].
    pub fn build(self) -> Arc<RenderPipeline> {
        let lbl = self.label.clone();
        let state = self.state;
        let multisample = self.multisample;
        let primitive = self.primitive;
        let depth_stencil = self.depth_stencil;
        let layout = self.layout;
        let vert_ref = state.shader_map.get(&self.vertex).unwrap();
        let vert_main = vert_ref.0.as_ref().unwrap().entry_point.clone();
        let vertex = Some(ShaderRef(vert_ref));
        let frag_tag = self.fragment.as_ref().map(|i| i.0);
        let vert_tag = self.vertex;
        let fragment = self.fragment.map(|i| {
            let frag_ref = state.shader_map.get(&i.0).unwrap();
            (
                frag_ref.0.as_ref().unwrap().entry_point.clone(),
                ShaderRef(frag_ref),
                i.1,
            )
        });
        let (fragment, frag_data) = match fragment {
            Some((tag, r, targets)) => (Some(r), Some((tag, targets))),
            None => (None, None),
        };
        let tags = ShaderTags::render(vert_tag, frag_tag);
        let manufacturer = Box::new(move |dev: &wgpu::Device, shaders: ShaderSet| {
            let label: Option<&str> = lbl.as_ref().map(|i| i.as_str());
            dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label,
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shaders.vertex.unwrap(),
                    entry_point: vert_main.as_str(),
                    buffers: &[],
                },
                fragment: shaders.fragment.as_ref().map(|frag| {
                    let (main, targets) = frag_data.as_ref().unwrap();
                    wgpu::FragmentState {
                        module: frag,
                        entry_point: main,
                        targets: targets.as_slice(),
                    }
                }),
                primitive: primitive.clone(),
                depth_stencil: depth_stencil.clone(),
                multisample: multisample.clone(),
            }).into()
        }) as Manufacturer;
        let ret = Arc::new((manufacturer)(
            &state.device,
            ShaderSet { vertex, fragment, compute: None },
        ).render());
        let val = ManufacturingData::new(manufacturer, tags, Arc::downgrade(&ret) as Weak<dyn Send + Sync>);
        state.manufacturers.write().unwrap().push(val);
        ret
    }
}

pub struct ComputePipelineBuilder<'a> {
    state: &'a ShaderState,
    label: Option<String>,
    layout: Option<PipelineLayout>,
    module: u128,
}

impl<'a> ComputePipelineBuilder<'a> {
    fn build(self) -> Arc<ComputePipeline> {
        let state = self.state;
        let label = self.label;
        let layout = self.layout;
        let module = self.module;
        let comp_ref = state.shader_map.get(&self.module).unwrap();
        let entry_point = comp_ref.0.as_ref().unwrap().entry_point.clone();
        let set = ShaderSet { vertex: None, fragment: None, compute: Some(ShaderRef(comp_ref))};
        let manufacturer = Box::new(move |dev: &wgpu::Device, shaders: ShaderSet| {
            dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: label.as_deref(),
                layout: layout.as_ref(),
                module: &shaders.compute.unwrap(),
                entry_point: entry_point.as_str(),
            }).into()
        }) as Manufacturer;
        let tags = ShaderTags::compute(module);
        let ret = Arc::new((manufacturer)(&state.device, set).compute());
        let val = ManufacturingData::new(manufacturer, tags, Arc::downgrade(&ret) as Weak<dyn Send + Sync>);
        state.manufacturers.write().unwrap().push(val);
        ret
    }
}

pub trait ToVec<T> {
    fn to_vec(self) -> Vec<T>;
}

impl<T> ToVec<T> for Vec<T> {
    fn to_vec(self) -> Vec<T> {
        self
    }
}

impl<T> ToVec<T> for T {
    fn to_vec(self) -> Vec<T> {
        vec![self]
    }
}

impl<T, const N: usize> ToVec<T> for [T; N] {
    fn to_vec(self) -> Vec<T> {
        self.into()
    }
}
