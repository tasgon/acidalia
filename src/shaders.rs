use iced_wgpu::wgpu;
use shaderc;
use std::collections::HashMap;
use std::path::Path;

use crate::graphics::GraphicsState;

pub struct ShaderState<T> {
    compiler: shaderc::Compiler,
    shader_map: HashMap<T, shaderc::CompilationArtifact>,
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
        src: impl AsRef<Path>,
        entry: &str,
        kind: shaderc::ShaderKind,
        options: &Option<shaderc::CompileOptions>,
    ) {
        let path: &Path = src.as_ref();
        let filename = path
            .file_name()
            .unwrap()
            .to_str()
            .expect("Invalid file name!");
        let data =
            std::fs::read_to_string(path).expect(&format!("Unable to read from {}!", filename));
        let res = self
            .compiler
            .compile_into_spirv(&data, kind, filename, entry, options.as_ref())
            .unwrap();
        self.shader_map.insert(key, res);
    }

    pub fn get_artifact(&self, key: impl AsRef<T>) -> Option<&shaderc::CompilationArtifact> {
        self.shader_map.get(key.as_ref())
    }

    pub fn get(&self, key: impl AsRef<T>) -> Option<&[u32]> {
        Some(self.get_artifact(key)?.as_binary())
    }

    pub fn create_shader(
        &self,
        key: impl AsRef<T>,
        gs: &mut crate::graphics::GraphicsState,
    ) -> Option<wgpu::ShaderModule> {
        let source = wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::SpirV(self.get(key)?.into()),
            flags: wgpu::ShaderFlags::default(),
        };
        // TODO: maybe store the shader modules instead of the artifacts?
        Some(gs.device.create_shader_module(&source))
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
        
    }
}