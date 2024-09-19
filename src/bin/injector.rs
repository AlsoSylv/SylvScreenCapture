//! TODO: Setup IPC
//!

use egui::{Color32, ColorImage, Frame, Image, TextureOptions};
use std::env;
use std::ffi::{CStr, CString};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};
use windows::core::{s, w, Interface, PCSTR};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Texture2D, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_RESOURCE_MISC_SHARED,
    D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGIResource1, DXGI_PRESENT, DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE,
    DXGI_SWAP_CHAIN_FLAG,
};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
use windows::Win32::System::Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE};
use windows::Win32::System::Threading::{
    CreateRemoteThread, OpenProcess, PROCESS_CREATE_THREAD, PROCESS_QUERY_INFORMATION,
    PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
mod dx11;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window = winit::window::WindowBuilder::new()
        .with_title("dx-11-text")
        .with_inner_size(PhysicalSize::new(600, 600))
        .build(&event_loop)
        .unwrap();

    let handle = window.window_handle().unwrap();

    let RawWindowHandle::Win32(hwnd) = handle.as_raw() else {
        panic!()
    };

    let PhysicalSize { width, height } = window.inner_size();

    let (swap_chain, device, context) = dx11::create_device_and_swap_chain(width, height, &hwnd);

    let mut render_target = None;

    unsafe {
        device
            .CreateRenderTargetView(
                &swap_chain.GetBuffer::<ID3D11Texture2D>(0).unwrap(),
                None,
                Some(&mut render_target),
            )
            .unwrap()
    };

    let egui_ctx = egui::Context::default();
    let mut egui_renderer = egui_directx11::Renderer::new(&device).unwrap();
    let mut egui_winit = egui_winit::State::new(
        egui_ctx.clone(),
        egui_ctx.viewport_id(),
        &window,
        None,
        None,
    );

    let mut texture: Option<ID3D11Texture2D> = None;

    unsafe {
        device
            .CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: 1920,
                    Height: 1080,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32
                        | D3D11_BIND_RENDER_TARGET.0 as u32,
                    CPUAccessFlags: 0,
                    MiscFlags: D3D11_RESOURCE_MISC_SHARED.0 as u32
                        | D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0 as u32,
                    MipLevels: 1,
                    ArraySize: 1,
                },
                None,
                Some(&mut texture),
            )
            .unwrap()
    };

    let texture = texture.unwrap();
    let resource = texture.cast::<IDXGIResource1>().unwrap();

    let maybe_handle = unsafe {
        resource
            .CreateSharedHandle(
                None,
                DXGI_SHARED_RESOURCE_READ.0 | DXGI_SHARED_RESOURCE_WRITE.0,
                w!("Sylvia's_Shared_Texture"),
            )
            .unwrap()
    };

    let mut description: D3D11_TEXTURE2D_DESC = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut description) };

    description.MiscFlags = 0;
    description.Usage.0 = D3D11_USAGE_STAGING.0;
    description.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    description.BindFlags = 0;
    let mut new_texture: Option<ID3D11Texture2D> = None;

    unsafe {
        device
            .CreateTexture2D(&description, None, Some(&mut new_texture))
            .unwrap()
    };

    let new_texture = new_texture.unwrap();

    println!("{maybe_handle:?}");

    println!("H");
    if env::args().nth(1).is_none() {
        println!("Please pass the process name as first argument");
        return;
    }
    let process_name = env::args().nth(1).unwrap();
    inject(&process_name).expect("AAA");

    std::thread::sleep(Duration::from_secs(2));

    let mut texture_handle = egui_ctx.load_texture(
        "RawDXOut",
        Arc::new(ColorImage::default()),
        TextureOptions::default(),
    );

    event_loop
        .run(|event, event_loop| match event {
            Event::AboutToWait => window.request_redraw(),
            Event::WindowEvent { window_id, event } => {
                if window_id != window.id() {
                    return;
                }

                if egui_winit.on_window_event(&window, &event).consumed {
                    return;
                }

                match event {
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::Resized(PhysicalSize { width, height }) => {
                        render_target.take();
                        unsafe {
                            swap_chain
                                .ResizeBuffers(
                                    0,
                                    width,
                                    height,
                                    DXGI_FORMAT_UNKNOWN,
                                    DXGI_SWAP_CHAIN_FLAG(0),
                                )
                                .unwrap();
                            let back_buffer = swap_chain.GetBuffer::<ID3D11Texture2D>(0).unwrap();
                            let mut new_render_target = None;
                            device
                                .CreateRenderTargetView(
                                    &back_buffer,
                                    None,
                                    Some(&mut new_render_target),
                                )
                                .unwrap();

                            render_target = new_render_target;
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        if let Some(render_target) = &render_target {
                            let input = egui_winit.take_egui_input(&window);
                            let output = egui_ctx.run(input, |ctx| {
                                egui::SidePanel::new(egui::panel::Side::Left, "new_side_panel")
                                    .frame(Frame::none().fill(Color32::WHITE))
                                    .resizable(false)
                                    .default_width(150.0)
                                    .show(ctx, |ui| ui.label("New Text here!!!"));

                                unsafe { context.CopyResource(&new_texture, &texture) };

                                let mut mapped_surface = D3D11_MAPPED_SUBRESOURCE::default();

                                if let Err(e) = unsafe {
                                    context.Map(
                                        &new_texture,
                                        0,
                                        D3D11_MAP_READ,
                                        0,
                                        Some(&mut mapped_surface),
                                    )
                                } {
                                    println!("Error reading mapped surface: {e}");
                                    return;
                                };

                                let slice = unsafe {
                                    std::slice::from_raw_parts(
                                        mapped_surface.pData as *const u8,
                                        description.Width as usize
                                            * description.Height as usize
                                            * 4,
                                    )
                                };

                                let image = ColorImage::from_rgba_unmultiplied([1920, 1080], slice);

                                texture_handle.set(image, TextureOptions::default());

                                egui::CentralPanel::default().show(ctx, |ui| {
                                    let image =
                                        Image::from_texture(&texture_handle).shrink_to_fit();
                                    ui.add(image);
                                });
                            });

                            let (render_output, platform_output, _) =
                                egui_directx11::split_output(output);

                            egui_winit.handle_platform_output(&window, platform_output);

                            unsafe {
                                context.ClearRenderTargetView(render_target, &[0.0, 0.0, 0.0, 1.0]);
                            }

                            egui_renderer
                                .render(
                                    &context,
                                    &render_target,
                                    &egui_ctx,
                                    render_output,
                                    window.scale_factor() as _,
                                )
                                .unwrap();

                            unsafe {
                                swap_chain.Present(1, DXGI_PRESENT(0)).unwrap();
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .unwrap();
}

fn inject(process_name: &str) -> Result<(), ()> {
    const KERNEL_32_DLL: PCSTR = s!("kernel32.dll");
    const LOAD_LIBRARY_A_C: PCSTR = s!("LoadLibraryA");

    let system =
        System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));
    let process = system
        .processes_by_name(process_name.as_ref())
        .next()
        .unwrap();

    let pid = process.pid().as_u32();

    let process_attach_rights = PROCESS_CREATE_THREAD
        | PROCESS_QUERY_INFORMATION
        | PROCESS_VM_OPERATION
        | PROCESS_VM_READ
        | PROCESS_VM_WRITE;

    unsafe {
        let process_handle = OpenProcess(process_attach_rights, false, pid).unwrap();

        if process_handle.0.is_null() {
            return Err(());
        }

        let module = GetModuleHandleA(KERNEL_32_DLL).unwrap();
        let load_library_ptr = GetProcAddress(module, LOAD_LIBRARY_A_C).unwrap();

        let mut dll_path = env::current_exe().unwrap();
        dll_path.pop();
        dll_path.push("screenshot_frame.dll");

        let dll_name_str = dll_path.to_str().unwrap();
        let dll_path = CString::new(dll_path.to_str().unwrap()).unwrap();

        let virtual_alloc = VirtualAllocEx(
            process_handle,
            None,
            dll_name_str.len() + 1,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );

        WriteProcessMemory(
            process_handle,
            virtual_alloc,
            dll_path.as_ptr().cast(),
            dll_name_str.len() + 1,
            None,
        )
        .unwrap();
        CreateRemoteThread(
            process_handle,
            None,
            0,
            Some(std::mem::transmute(load_library_ptr)),
            Some(virtual_alloc),
            0,
            None,
        )
        .unwrap();
    }

    Ok(())
}
