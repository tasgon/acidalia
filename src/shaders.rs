use crate::wgpu;
use acidalia_core::Nametag;
use shaderc;
use std::collections::HashMap;
use std::path::Path;
use acidalia_proc_macros::Nametag;

use crate::graphics::GraphicsState;

/// A struct that provides shader compilation and access from the program.
/// Utilizes `shaderc` to compile GLSL source into SPIR-V.
pub struct ShaderState {
    compiler: shaderc::Compiler,
    shader_map: HashMap<u128, wgpu::ShaderModule>,
}

impl ShaderState {
    /// Construct a new `ShaderState`
    pub fn new() -> Self {
        let compiler = shaderc::Compiler::new().unwrap();
        let shader_map = HashMap::new();
        Self {
            compiler,
            shader_map,
        }
    }

    /// Loads a shader from a file. In the future, shaders reloaded from here
    /// will have hot-reloading.
    pub fn load_src(
        &mut self,
        key: impl Nametag,
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
        self.shader_map
            .insert(key.tag(), gs.device.create_shader_module(&desc));
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
        let desc = wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::SpirV(res.as_binary().into()),
            flags: wgpu::ShaderFlags::default(),
        };
        self.shader_map
            .insert(key.tag(), gs.device.create_shader_module(&desc));
    }

    /// Attempt to retrieve a shader with a given tag `key`.
    pub fn get(&self, key: impl Nametag) -> Option<&wgpu::ShaderModule> {
        Some(self.shader_map.get(&key.tag())?)
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
}

/// The key enums for the internal shaders.
#[derive(Nametag)]
pub enum InternalShaders {
    IcedVert,
    IcedFrag,
}