use ash::vk;
use log::info;
use std::sync::Arc;
use winit::raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;
// Fix the extension imports with proper lowercase paths and aliases
#[cfg(target_os = "windows")]
use ash::khr::win32_surface as Win32Surface;

#[cfg(all(unix, not(target_os = "android"), not(target_os = "macos")))]
use ash::khr::xlib_surface as XlibSurface; // Changed from XlibSurface

#[cfg(target_os = "macos")]
use ash::mvk::macos_surface as MacOSSurface;

use ash::ext::debug_utils as DebugUtils; // Changed from DebugUtils
use ash::ext::debug_utils;
use ash::khr::surface as Surface; // Changed from Surface
use ash::khr::surface;
use ash::khr::wayland_surface;
use ash::khr::xlib_surface;
// Rest of your existing imports...
#[cfg(target_os = "macos")]
use cocoa::appkit::{NSView, NSWindow};
#[cfg(target_os = "macos")]
use cocoa::base::id as cocoa_id;
#[cfg(target_os = "macos")]
use metal::CoreAnimationLayer;
#[cfg(target_os = "macos")]
use objc::runtime::YES;

// Required extension names ------------------------------------------------------
#[cfg(target_os = "macos")]
pub fn required_extension_names() -> Vec<*const i8> {
    vec![
        Surface::NAME.as_ptr(),
        MacOSSurface::NAME.as_ptr(),
        DebugUtils::NAME.as_ptr(),
    ]
}

#[cfg(target_os = "windows")]
pub fn required_extension_names() -> Vec<*const i8> {
    vec![
        Surface::NAME.as_ptr(),
        Win32Surface::NAME.as_ptr(),
        DebugUtils::NAME.as_ptr(),
    ]
}

#[cfg(all(unix, not(target_os = "android"), not(target_os = "macos")))]
pub fn required_extension_names() -> Vec<*const i8> {
    let mut extensions = vec![
        surface::NAME.as_ptr(),      // VK_KHR_surface
        xlib_surface::NAME.as_ptr(), // VK_KHR_xlib_surface for X11
        debug_utils::NAME.as_ptr(),  // VK_EXT_debug_utils
    ];

    // Check if WAYLAND_DISPLAY env var exists
    if std::env::var_os("WAYLAND_DISPLAY")
        .map(|val| !val.is_empty())
        .unwrap_or(false)
    {
        info!("WAYLAND_DISPLAY env var exists");
        extensions.push(wayland_surface::NAME.as_ptr()); // VK_KHR_wayland_surface for Wayland
    }

    extensions
}

// Create surface ---------------------------------------------------------
#[cfg(all(unix, not(target_os = "android"), not(target_os = "macos")))]
pub unsafe fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: Arc<Window>,
) -> Result<vk::SurfaceKHR, vk::Result> {
    use ash::vk::{WaylandSurfaceCreateInfoKHR, XlibSurfaceCreateInfoKHR};
    use std::{ptr, sync::Arc};
    use winit::{
        raw_window_handle::{RawDisplayHandle, RawWindowHandle},
        window::Window,
    };
    let display_handle = window
        .raw_display_handle()
        .expect("Failed to get display handle");
    let window_handle = window
        .raw_window_handle()
        .expect("Failed to get window handle");

    match (display_handle, window_handle) {
        // X11 Surface Creation
        (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(window)) => {
            let display_ptr = display
                .display
                .map_or(ptr::null_mut(), |nn| nn.as_ptr() as *mut _);
            let x11_create_info = XlibSurfaceCreateInfoKHR {
                s_type: vk::StructureType::XLIB_SURFACE_CREATE_INFO_KHR,
                p_next: ptr::null(),
                flags: vk::XlibSurfaceCreateFlagsKHR::empty(),
                window: window.window,
                dpy: display_ptr,
                _marker: std::marker::PhantomData,
            };
            let xlib_surface_loader = ash::khr::xlib_surface::Instance::new(entry, instance);
            unsafe { xlib_surface_loader.create_xlib_surface(&x11_create_info, None) }
        }

        // Wayland Surface Creation
        (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
            let wayland_create_info = WaylandSurfaceCreateInfoKHR {
                s_type: vk::StructureType::WAYLAND_SURFACE_CREATE_INFO_KHR,
                p_next: ptr::null(),
                flags: vk::WaylandSurfaceCreateFlagsKHR::empty(),
                display: display.display.as_ptr(),
                surface: window.surface.as_ptr(),
                _marker: std::marker::PhantomData,
            };
            let wayland_surface_loader = ash::khr::wayland_surface::Instance::new(entry, instance);
            unsafe { wayland_surface_loader.create_wayland_surface(&wayland_create_info, None) }
        }

        // Unsupported display/window combination
        _ => panic!("Unsupported display or window handle type (expected Xlib or Wayland)"),
    }
}

#[cfg(target_os = "macos")]
pub unsafe fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &winit::window::Window,
) -> Result<vk::SurfaceKHR, vk::Result> {
    use std::mem;
    use std::os::raw::c_void;
    use std::ptr;
    use winit::platform::macos::WindowExtMacOS;

    let wnd: cocoa_id = mem::transmute(window.ns_window());

    let layer = CoreAnimationLayer::new();

    layer.set_edge_antialiasing_mask(0);
    layer.set_presents_with_transaction(false);
    layer.remove_all_animations();

    let view = wnd.contentView();

    layer.set_contents_scale(view.backingScaleFactor());
    view.setLayer(mem::transmute(layer.as_ref()));
    view.setWantsLayer(YES);

    let create_info = vk::MacOSSurfaceCreateInfoMVK {
        s_type: vk::StructureType::MACOS_SURFACE_CREATE_INFO_MVK,
        p_next: ptr::null(),
        flags: Default::default(),
        p_view: window.ns_view() as *const c_void,
    };

    let macos_surface_loader = MacOSSurface::new(entry, instance);
    macos_surface_loader.create_macos_surface(&create_info, None)
}

#[cfg(target_os = "windows")]
pub unsafe fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: Arc<Window>, // Changed from Arc<Window> to &Window
) -> Result<vk::SurfaceKHR, vk::Result> {
    use std::ptr;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use winit::platform::windows::WindowExtWindows; // Add this for hwnd access

    // Get the HWND using WindowExtWindows trait
    let hwnd = HWND(window.hwnd() as isize); // Get hwnd through the extension trait

    let hinstance = GetModuleHandleW(None).unwrap();

    let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
        s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
        p_next: ptr::null(),
        flags: Default::default(),
        hinstance: hinstance.0 as *const _,
        hwnd: hwnd.0 as *const _,
        ..Default::default() // This handles the _marker field and other defaults
    };

    let win32_surface_loader = Win32Surface::new(entry, instance);
    win32_surface_loader.create_win32_surface(&win32_create_info, None)
}
