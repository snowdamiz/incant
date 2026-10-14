//! Android API26+ device host. No Android backend is selected on other platforms.
#[cfg(target_os = "android")]
mod host;
#[cfg(target_os = "android")]
mod jni;
#[cfg(any(target_os = "android", test))]
mod lifecycle;
#[cfg(target_os = "android")]
mod surface;
