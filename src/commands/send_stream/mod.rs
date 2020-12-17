#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(target_os = "linux"))]
pub mod other;
#[cfg(not(target_os = "linux"))]
pub use other::*;
