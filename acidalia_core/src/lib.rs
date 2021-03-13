/// The core element in registering shaders for use in the program.
/// Applying `#[derive(Nametag)]` to an enum with variants will let you use those enums
/// as identifiers for shaders in your pipeline.
pub trait Nametag {
    fn tag(self) -> u128;
}

impl Nametag for u128 {
    fn tag(self) -> u128 {
        self
    }
}
