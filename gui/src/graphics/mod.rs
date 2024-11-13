#[cfg_attr(target_os = "macos", path = "metal.rs")]
#[cfg_attr(not(target_os = "macos"), path = "vulkan.rs")]
mod api;

#[cfg(not(target_os = "macos"))]
pub type DefaultApi = self::api::Vulkan;

#[cfg(target_os = "macos")]
pub type DefaultApi = self::api::Metal;

pub trait GraphicsApi: Sized + 'static {
    type PhysicalDevice: PhysicalDevice;

    type InitError: core::error::Error;

    fn init() -> Result<Self, Self::InitError>;

    fn enumerate_physical_devices(&self) -> &[Self::PhysicalDevice];
}

pub trait PhysicalDevice: Sized {
    fn name(&self) -> &str;
}
