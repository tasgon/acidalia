use shaderc;
use std::collections::HashMap;
use std::path::Path;

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

    pub fn load_src(&mut self, key: T, src: impl AsRef<Path>, entry: &str, kind: shaderc::ShaderKind, options: &Option<shaderc::CompileOptions>) {
        let path: &Path = src.as_ref();
        let filename = path.file_name().unwrap().to_str().expect("Invalid file name!");
        let data = std::fs::read_to_string(path).expect(&format!("Unable to read from {}!", filename));
        let res = self.compiler.compile_into_spirv(&data, kind, filename, entry, options.as_ref()).unwrap();
        self.shader_map.insert(key, res);
    }

    pub fn get_artifact(&self, key: impl AsRef<T>) -> Option<&shaderc::CompilationArtifact> {
        self.shader_map.get(key.as_ref())
    }

    pub fn get(&self, key: impl AsRef<T>) -> Option<&[u32]> {
        Some(self.get_artifact(key)?.as_binary())
    }

    pub fn create_shader(&self, key: impl AsRef<T>, gfx_state: &mut crate::graphics::GraphicsState) -> Option<wgpu::ShaderModule> {
        Some(gfx_state.device.create_shader_module(self.get(key)?))
    }
}