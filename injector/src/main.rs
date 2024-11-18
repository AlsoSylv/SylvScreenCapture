use egui::{Color32, ColorImage, Frame, Image, TextureHandle, TextureOptions};
use shmem::RenderingAPI;
// use interprocess::local_socket::{GenericNamespaced, Listener, ListenerOptions, ToNsName};
use std::env;
use std::ffi::OsStr;
// use std::io::{Read, Write};
use std::sync::Arc;
use sysinfo::{Process, ProcessRefreshKind, RefreshKind, System};
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11Texture2D,
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_RESOURCE_MISC_SHARED,
    D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    IDXGIResource, IDXGIResource1, IDXGISwapChain, DXGI_PRESENT, DXGI_SHARED_RESOURCE_READ,
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

fn d3d11_texture_description(width: u32, height: u32, nt_handle: bool) -> D3D11_TEXTURE2D_DESC {
    let mut flags = D3D11_RESOURCE_MISC_SHARED.0 as u32;

    if nt_handle {
        flags |= D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0 as u32;
    }

    D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32 | D3D11_BIND_RENDER_TARGET.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: flags,
        MipLevels: 1,
        ArraySize: 1,
    }
}

#[derive(Default)]
struct App {
    window: Option<Window>,
    egui_state: Option<EguiState>,
    new_texture: Option<ID3D11Texture2D>,
    // shared_handle: Option<HANDLE>,
    // listener: Option<Listener>,
    shared_memory: Option<shmem::Shmem>,
    d3d11_state: Option<D3D11State>,
    program_state: Option<Vec<ProgramState>>,
}

struct ProgramState {
    name: String,
    pid: u32,
    shared_memory: shmem::Shmem,
    copy_buffer: ID3D11Texture2D,
    textures: [Option<ID3D11Texture2D>; 2],
}

struct EguiState {
    winit: egui_winit::State,
    renderer: egui_directx11::Renderer,
    ctx: egui::Context,
    texture_handle: TextureHandle,
}

struct D3D11State {
    ctx: ID3D11DeviceContext,
    render_target: Option<ID3D11RenderTargetView>,
    swap_chain: IDXGISwapChain,
    device: ID3D11Device,
    textures: Vec<(u32, Option<ID3D11Texture2D>, Option<ID3D11Texture2D>)>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut textures = Vec::new();

        let system = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::new()),
        );

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
                    &d3d11_texture_description(1920, 1080, false),
                    None,
                    Some(&mut texture),
                )
                .unwrap()
        };

        let texture = texture.unwrap();
        let resource = texture.cast::<IDXGIResource>().unwrap();

        let maybe_handle = unsafe { resource.GetSharedHandle().unwrap() };

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

        let process = system
            .processes_by_name(OsStr::new(&process_name))
            .next()
            .unwrap();

        // let opts = ListenerOptions::new()
        //     .name(
        //         r"\\.\pipe\sylvias_shared_handle.sock"
        //             .to_ns_name::<GenericNamespaced>()
        //             .unwrap(),
        //     )
        //     .nonblocking(interprocess::local_socket::ListenerNonblockingMode::Accept);
        // let listener = opts.create_sync().unwrap();

        let current_monitor_size = window.current_monitor().unwrap().size();

        let monitor_size = (current_monitor_size.width * current_monitor_size.height) as usize;

        let mut shared_mem = shmem::ShmemBuilder::new("SylvScreenShare")
            .size(4 * monitor_size)
            .create()
            .unwrap();
        shared_mem.set_owner();

        let [dx10_down_texture, dx11_up_texture] =
            inject(process, &device, &shared_mem).expect("AAA");
        textures.push((process.pid().as_u32(), dx10_down_texture, dx11_up_texture));

        let state = Self {
            window: Some(window),
            egui_state: Some(EguiState {
                ctx: egui_ctx,
                winit: egui_winit,
                renderer: egui_renderer,
                texture_handle,
            }),
            program_state: Some(Vec::new()),
            new_texture: Some(new_texture),
            // listener: Some(listener),
            // shared_handle: Some(shared_handle),
            shared_memory: Some(shared_mem),
            d3d11_state: Some(D3D11State {
                ctx: context,
                device,
                render_target,
                swap_chain,
                textures,
            }),
        };

        *self = state;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let egui = self.egui_state.as_mut().unwrap();
        let window = self.window.as_mut().unwrap();
        let new_texture = self.new_texture.as_mut().unwrap();
        let shared_mem = self.shared_memory.as_mut().unwrap();
        let program_state = self.program_state.as_mut().unwrap();
        // let shared_handle = self.shared_handle.as_mut().unwrap();
        // let listener = self.listener.as_mut().unwrap();
        let d3d11_state = self.d3d11_state.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                d3d11_state.render_target.take();

                dx11::resize_back_buffer(&d3d11_state.swap_chain, width, height).unwrap();

                unsafe {
                    let back_buffer = d3d11_state
                        .swap_chain
                        .GetBuffer::<ID3D11Texture2D>(0)
                        .unwrap();
                    let mut new_render_target = None;
                    d3d11_state
                        .device
                        .CreateRenderTargetView(&back_buffer, None, Some(&mut new_render_target))
                        .unwrap();

                    d3d11_state.render_target = new_render_target;
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(render_target) = &d3d11_state.render_target {
                    let input = egui.winit.take_egui_input(window);
                    let output = egui.ctx.run(input, |ctx| {
                        egui::SidePanel::new(egui::panel::Side::Left, "new_side_panel")
                            .frame(Frame::none().fill(Color32::WHITE))
                            .resizable(false)
                            .default_width(150.0)
                            .show(ctx, |ui| {
                                let button = ui.button("Record Program");
                                if button.clicked() {

                                    // TODO: Enum windows
                                }
                                ui.label("New Text here!!!")
                            });

                        for program in program_state.iter_mut() {
                            let header = program.shared_memory.header();
                            let in_use_texture = if header.nt_handle_in_use() {
                                &program.textures[0]
                            } else {
                                &program.textures[1]
                            };

                            if let Some(texture) = in_use_texture {
                                unsafe {
                                    d3d11_state.ctx.CopyResource(&program.copy_buffer, texture);
                                }
                                // TODO: Display Texture
                            }
                        }

                        let header = shared_mem.header();
                        let (_pid, dx10_down_texture, dx11_up_texutre) = &d3d11_state.textures[0];

                        let in_use_texture = if header.nt_handle_in_use() {
                            dx11_up_texutre
                        } else {
                            dx10_down_texture
                        }
                        .as_ref();

                        // TODO: Check process is in use

                        let image = if let Some(in_use_texture) = in_use_texture {
                            unsafe { d3d11_state.ctx.CopyResource(&*new_texture, in_use_texture) };
                            let mut mapped_surface = D3D11_MAPPED_SUBRESOURCE::default();
                            if let Err(e) = unsafe {
                                d3d11_state.ctx.Map(
                                    &*new_texture,
                                    0,
                                    D3D11_MAP_READ,
                                    0,
                                    Some(&mut mapped_surface),
                                )
                            } {
                                println!("Error reading mapped surface: {e}");
                                return;
                            };

                            let (width, height) = header.get_width_and_height();

                            let slice = unsafe {
                                std::slice::from_raw_parts(
                                    mapped_surface.pData as *const u8,
                                    width as usize * height as usize * 4,
                                )
                            };

                            // if !slice.is_empty() {
                            //     println!("{:?}", &slice[0..4])
                            // }

                            let image = if (width as usize | height as usize) == 0
                                || header.api() == RenderingAPI::None
                            {
                                ColorImage::from_rgba_unmultiplied([1, 1], &[0, 0, 0, 255])
                            } else if header.ignore_alpha() {
                                ColorImage {
                                    size: [width as usize, height as usize],
                                    pixels: slice
                                        .chunks(4)
                                        .map(|slice| {
                                            Color32::from_rgb(slice[2], slice[1], slice[0])
                                        })
                                        .collect(),
                                }
                            } else {
                                ColorImage::from_rgba_unmultiplied(
                                    [width as usize, height as usize],
                                    slice,
                                )
                            };

                            image
                        } else {
                            ColorImage::from_rgba_unmultiplied([1, 1], &[0, 0, 0, 255])
                        };
                        egui.texture_handle.set(image, TextureOptions::default());

                        egui::CentralPanel::default().show(ctx, |ui| {
                            let image = Image::from_texture(&egui.texture_handle).shrink_to_fit();
                            ui.add(image);
                        });

                        unsafe { d3d11_state.ctx.Unmap(&*new_texture, 0) };
                    });

                    let (render_output, platform_output, _) = egui_directx11::split_output(output);

                    egui.winit.handle_platform_output(window, platform_output);

                    unsafe {
                        d3d11_state
                            .ctx
                            .ClearRenderTargetView(render_target, &[0.0, 0.0, 0.0, 1.0]);
                    }

                    if let Err(e) = egui.renderer.render(
                        &d3d11_state.ctx,
                        render_target,
                        &egui.ctx,
                        render_output,
                        window.scale_factor() as _,
                    ) {
                        println!("{e}");
                        return;
                    };

                    unsafe {
                        d3d11_state.swap_chain.Present(1, DXGI_PRESENT(0)).unwrap();
                    }

                    window.request_redraw();
                }
            }
            _ => {}
        }

        // if let Some(Ok(_listener)) = listener.next() {
        //     todo!("lol wtf")
        // }
    }
}

fn inject(
    process: &Process,
    device: &ID3D11Device,
    shared_memory: &shmem::Shmem,
) -> Result<[Option<ID3D11Texture2D>; 2], ()> {
    const SHARED_RIGHTS: u32 = DXGI_SHARED_RESOURCE_READ.0 | DXGI_SHARED_RESOURCE_WRITE.0;
    const NT_HANDLE_APIS: &[&str] = &["d3d11.dll", "d3d12.dll", "opengl32.dll", "vulkan-1.dll"];
    const HANDLE_APIS: &[&str] = &["d3d9.dll", "d3d10.dll"];

    let target_process = process_ext::Process::new_with_system(process);
    let mut textures = [None, None];
    let mut nt_handle = false;
    let mut handle = false;

    target_process
        .iter_modules(|ostr| {
            let dll_name = std::path::Path::new(ostr).file_name().unwrap();

            for api in NT_HANDLE_APIS {
                if dll_name == *api {
                    nt_handle |= true;
                    break;
                }
            }

            for api in HANDLE_APIS {
                if dll_name == *api {
                    handle |= true;
                    break;
                }
            }

            println!("{dll_name:?}");
        })
        .unwrap();

    let current_process = process_ext::Process::current_process();

    if handle {
        if let Some(texture) = create_texture(device, 1920, 1080, false) {
            let resource = texture.cast::<IDXGIResource>().unwrap();

            let handle = unsafe { resource.GetSharedHandle().unwrap() };
            shared_memory.header().set_shared_handle(handle.0);

            textures[0] = Some(texture);
        }
    };

    if nt_handle {
        if let Some(texture) = create_texture(device, 1920, 1080, true) {
            let resource = texture.cast::<IDXGIResource1>().unwrap();
            let handle = unsafe {
                resource
                    .CreateSharedHandle(None, SHARED_RIGHTS, None)
                    .unwrap()
            };
            let dup_handle = unsafe {
                current_process
                    .duplicate_handle(&target_process, handle, SHARED_RIGHTS)
                    .unwrap()
            };

            shared_memory.header().set_nt_shared_handle(dup_handle.0);

            textures[1] = Some(texture);
        }
    }

    let mut dll_path = env::current_exe().unwrap();
    dll_path.pop();
    dll_path.push("screen_recorder.dll");

    unsafe { target_process.load_remote_library(&dll_path) }.unwrap();

    Ok(textures)
}

fn create_texture(
    device: &ID3D11Device,
    width: u32,
    height: u32,
    nt_handle: bool,
) -> Option<ID3D11Texture2D> {
    let mut texture = None;
    unsafe {
        device.CreateTexture2D(
            &d3d11_texture_description(width, height, nt_handle),
            None,
            Some(&mut texture),
        )
    }
    .unwrap();

    texture
}
