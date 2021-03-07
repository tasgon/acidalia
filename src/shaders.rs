use iced_wgpu::wgpu;
use shaderc;
use std::collections::HashMap;
use std::path::Path;

use crate::graphics::GraphicsState;

pub struct ShaderState<T> {
    compiler: shaderc::Compiler,
    shader_map: HashMap<T, wgpu::ShaderModule>,
}

impl<T: Eq + std::hash::Hash> ShaderState<T> {
    pub fn new() -> Self {
        let compiler = shaderc::Compiler::new().unwrap();
        let shader_map = HashMap::new();
        Self {
            compiler,
            shader_map,
        }
    }

    pub fn load_src(
        &mut self,
        key: T,
        path: impl AsRef<Path>,
        entry_point: &str,
        kind: shaderc::ShaderKind,
        options: Option<&shaderc::CompileOptions>,
        gs: &mut GraphicsState,
    ) {
        let path: &Path = path.as_ref();
        let filename = path
            .file_name()
            .unwrap()
            .to_str()
            .expect("Invalid file name!");
        let data =
            std::fs::read_to_string(path).expect(&format!("Unable to read from {}!", filename));
        let res = self
            .compiler
            .compile_into_spirv(&data, kind, filename, entry_point, options)
            .unwrap();
        let desc = wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::SpirV(res.as_binary().into()),
            flags: wgpu::ShaderFlags::default(),
        };
        self.shader_map.insert(key, gs.device.create_shader_module(&desc));
    }

    pub fn load_str(
        &mut self,
        key: T,
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
        let desc = wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::SpirV(res.as_binary().into()),
            flags: wgpu::ShaderFlags::default(),
        };
        self.shader_map.insert(key, gs.device.create_shader_module(&desc));
    }

    pub fn get(&self, key: &T) -> Option<&wgpu::ShaderModule> {
        Some(self.shader_map.get(key)?)
    }
}

trait AsShaderSrc {
    fn get_src(&mut self) -> String;
}

#[derive(PartialEq, Eq, Hash)]
pub enum InternalShaders {
    ICED_VERT,
    ICED_FRAG,
}

pub type InternalShaderState = ShaderState<InternalShaders>;

impl InternalShaderState {
    pub fn init_shaders(&mut self, gs: &mut GraphicsState) {
        self.load_str(InternalShaders::ICED_VERT, "iced.vert",include_str!("gl/iced.vert"), "main", shaderc::ShaderKind::Vertex, None, gs);
        self.load_str(InternalShaders::ICED_FRAG, "iced.frag", include_str!("gl/iced.frag"), "main", shaderc::ShaderKind::Fragment, None, gs);
        println!("Initialized shaders");
    }
}