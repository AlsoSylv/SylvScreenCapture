use egui::{Color32, ColorImage, Frame, Image, TextureHandle, TextureOptions};
use interprocess::local_socket::{GenericNamespaced, Listener, ListenerOptions, ToNsName};
use std::env;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};
use windows::core::Interface;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11Texture2D,
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_RESOURCE_MISC_SHARED,
    D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    IDXGIResource1, IDXGISwapChain, DXGI_PRESENT, DXGI_SHARED_RESOURCE_READ,
    DXGI_SHARED_RESOURCE_WRITE,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;
mod dx11;
mod process_ext;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();

    let mut app = App::default();

    event_loop.run_app(&mut app).unwrap();
}

#[repr(C)]
struct SharedMemoryHeader {
    width: u32,
    height: u32,
}

#[derive(Default)]
struct App {
    window: Option<Window>,
    d3d11_ctx: Option<ID3D11DeviceContext>,
    render_target: Option<ID3D11RenderTargetView>,
    swap_chain: Option<IDXGISwapChain>,
    device: Option<ID3D11Device>,
    egui_winit: Option<egui_winit::State>,
    renderer: Option<egui_directx11::Renderer>,
    egui_ctx: Option<egui::Context>,
    texture: Option<ID3D11Texture2D>,
    new_texture: Option<ID3D11Texture2D>,
    texture_handle: Option<TextureHandle>,
    shared_handle: Option<HANDLE>,
    listener: Option<Listener>,
    shared_memory: Option<shared_memory::Shmem>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let width = 600;
        let height = 600;
        let size = PhysicalSize::new(width, height);

        let window_attributes = Window::default_attributes()
            .with_title("dx-11-test")
            .with_inner_size(size);
        let window = event_loop.create_window(window_attributes).unwrap();

        let RawWindowHandle::Win32(win32_handle) = window.window_handle().unwrap().as_raw() else {
            panic!("Expected Win32 handle")
        };

        let (swap_chain, device, context) =
            dx11::create_device_and_swap_chain(width, height, &win32_handle);

        let egui_ctx = egui::Context::default();
        let egui_renderer = egui_directx11::Renderer::new(&device).unwrap();
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            &window,
            None,
            None,
            None,
        );

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
                    None,
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

        let texture_handle = egui_ctx.load_texture(
            "RawDXOut",
            Arc::new(ColorImage::default()),
            TextureOptions::default(),
        );

        println!("H");
        if env::args().nth(1).is_none() {
            println!("Please pass the process name as first argument");
            return;
        }
        let process_name = env::args().nth(1).unwrap();

        let opts = ListenerOptions::new()
            .name(
                r"\\.\pipe\sylvias_shared_handle.sock"
                    .to_ns_name::<GenericNamespaced>()
                    .unwrap(),
            )
            .nonblocking(interprocess::local_socket::ListenerNonblockingMode::Accept);

        let listener = opts.create_sync().unwrap();

        let shared_handle = inject(&process_name, maybe_handle).expect("AAA");

        let current_monitor_size = window.current_monitor().unwrap().size();

        let header_size = size_of::<SharedMemoryHeader>();
        let monitor_size = (current_monitor_size.width * current_monitor_size.height) as usize;
        let rgba_size = size_of::<u8>() * 4;
        let shared_memory_size = header_size + rgba_size * monitor_size;

        let shared_mem = shared_memory::ShmemConf::new()
            .os_id("SylvScreenShare")
            .size(shared_memory_size)
            .create()
            .unwrap();

        let state = Self {
            window: Some(window),
            d3d11_ctx: Some(context),
            render_target: render_target,
            device: Some(device),
            egui_ctx: Some(egui_ctx),
            egui_winit: Some(egui_winit),
            renderer: Some(egui_renderer),
            swap_chain: Some(swap_chain),
            new_texture: Some(new_texture),
            texture: Some(texture),
            texture_handle: Some(texture_handle),
            listener: Some(listener),
            shared_handle: Some(shared_handle),
            shared_memory: Some(shared_mem),
        };

        *self = state;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let swap_chain = self.swap_chain.as_mut().unwrap();
        let device = self.device.as_mut().unwrap();
        let egui_winit = self.egui_winit.as_mut().unwrap();
        let window = self.window.as_mut().unwrap();
        let d3d11_ctx = self.d3d11_ctx.as_mut().unwrap();
        let egui_renderer = self.renderer.as_mut().unwrap();
        let egui_ctx = self.egui_ctx.as_mut().unwrap();
        let new_texture = self.new_texture.as_mut().unwrap();
        let texture = self.texture.as_mut().unwrap();
        let shared_mem = self.shared_memory.as_mut().unwrap();
        let texture_handle = self.texture_handle.as_mut().unwrap();
        let shared_handle = self.shared_handle.as_mut().unwrap();
        let listener = self.listener.as_mut().unwrap();

        let header_size = size_of::<SharedMemoryHeader>();
        let rgba_size = size_of::<u8>() * 4;

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                self.render_target.take();

                dx11::resize_back_buffer(&swap_chain, width, height).unwrap();

                unsafe {
                    let back_buffer = swap_chain.GetBuffer::<ID3D11Texture2D>(0).unwrap();
                    let mut new_render_target = None;
                    device
                        .CreateRenderTargetView(&back_buffer, None, Some(&mut new_render_target))
                        .unwrap();

                    self.render_target = new_render_target;
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(render_target) = &self.render_target {
                    let input = egui_winit.take_egui_input(&window);
                    let output = egui_ctx.run(input, |ctx| {
                        egui::SidePanel::new(egui::panel::Side::Left, "new_side_panel")
                            .frame(Frame::none().fill(Color32::WHITE))
                            .resizable(false)
                            .default_width(150.0)
                            .show(ctx, |ui| ui.label("New Text here!!!"));

                        unsafe { d3d11_ctx.CopyResource(&*new_texture, &*texture) };
                        // let mut mapped_surface = D3D11_MAPPED_SUBRESOURCE::default();
                        // if let Err(e) = unsafe {
                        //     d3d11_ctx.Map(
                        //         &new_texture,
                        //         0,
                        //         D3D11_MAP_READ,
                        //         0,
                        //         Some(&mut mapped_surface),
                        //     )
                        // } {
                        //     println!("Error reading mapped surface: {e}");
                        //     return;
                        // };

                        let shared_ptr = shared_mem.as_ptr();
                        let header = unsafe { &*shared_ptr.cast::<SharedMemoryHeader>() };
                        let rgba_ptr = unsafe { shared_ptr.add(header_size) };

                        let buffer_size = (header.width * header.height) as usize;
                        let slice_size = buffer_size * rgba_size;

                        let slice = unsafe { std::slice::from_raw_parts(rgba_ptr, slice_size) };

                        // let slice = unsafe {
                        //     std::slice::from_raw_parts(
                        //         mapped_surface.pData as *const u8,
                        //         description.Width as usize
                        //             * description.Height as usize
                        //             * 4,
                        //     )
                        // };

                        let image = if (header.width as usize | header.height as usize) == 0 {
                            ColorImage::from_rgba_unmultiplied([1, 1], &[0, 0, 0, 255])
                        } else {
                            ColorImage::from_rgba_unmultiplied(
                                [header.width as usize, header.height as usize],
                                slice,
                            )
                        };

                        texture_handle.set(image, TextureOptions::default());

                        egui::CentralPanel::default().show(ctx, |ui| {
                            let image = Image::from_texture(&*texture_handle).shrink_to_fit();
                            ui.add(image);
                        });

                        // unsafe { d3d11_ctx.Unmap(&*new_texture, 0) };
                    });

                    let (render_output, platform_output, _) = egui_directx11::split_output(output);

                    egui_winit.handle_platform_output(&window, platform_output);

                    unsafe {
                        d3d11_ctx.ClearRenderTargetView(render_target, &[0.0, 0.0, 0.0, 1.0]);
                    }

                    egui_renderer
                        .render(
                            &d3d11_ctx,
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

                window.request_redraw();
            }
            _ => {}
        }

        if let Some(Ok(mut listener)) = listener.next() {
            let mut pid_buffer = [0; 4];
            listener.read_exact(&mut pid_buffer).unwrap();
            listener
                .write(&(shared_handle.0 as isize).to_le_bytes())
                .unwrap();
        }
    }
}

fn inject(process_name: &str, original_shared_handle: HANDLE) -> Result<HANDLE, ()> {
    const SHARED_RIGHTS: u32 = DXGI_SHARED_RESOURCE_READ.0 | DXGI_SHARED_RESOURCE_WRITE.0;

    let system =
        System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));

    let target_process = process_ext::Process::new_with_system(process_name, &system);
    let modules = target_process.get_modules().unwrap();
    modules.iter().map(PathBuf::from).for_each(|path| {
        let dll_name = path.file_name().unwrap();

        println!("{dll_name:?}");
    });

    let current_process = process_ext::Process::current_process();

    assert!(!target_process.handle().0.is_null());

    let shared_handle = unsafe {
        current_process.duplicate_handle(&target_process, original_shared_handle, SHARED_RIGHTS)
    }
    .unwrap();

    let mut dll_path = env::current_exe().unwrap();
    dll_path.pop();
    dll_path.push("screen_recorder.dll");

    unsafe { target_process.load_remote_library(&dll_path) }.unwrap();

    Ok(shared_handle)
}
