use egui::{Frame, Image, Layout, TextureId};
use encoder_abstraction::Config;
use shared_defs::SharedMemoryHeader;
use shmem::Shmem;
use std::ffi::{CStr, OsString};
use std::io::Write;
use std::time::Instant;
// use std::io::{Read, Write};
use sysinfo::{Pid, Process, ProcessRefreshKind, RefreshKind, System};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_RESOURCE_MISC_SHARED,
    D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, ID3D11Device,
    ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    DXGI_PRESENT, DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE, IDXGIAdapter,
    IDXGIResource, IDXGIResource1, IDXGISwapChain,
};
use windows::core::Interface;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::dx11::render_target;
mod dx11;
mod process_ext;
mod system_ext;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let mut app = WinitState::default();

    event_loop.run_app(&mut app).unwrap();
}

fn d3d11_texture_description(
    width: u32,
    height: u32,
    nt_handle: bool,
    shared: bool,
) -> D3D11_TEXTURE2D_DESC {
    let mut flags = if shared {
        D3D11_RESOURCE_MISC_SHARED.0 as u32
    } else {
        0
    };

    if shared && nt_handle {
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
struct WinitState {
    window: Option<Window>,
    egui_state: Option<EguiState>,
    app_state: Option<AppState>,
    // shared_handle: Option<HANDLE>,
    // listener: Option<Listener>,
    d3d11_state: Option<D3D11State>,
}

struct AppState {
    target_states: Vec<TargetState>,
    system: System,
    /// This is used to copy ALL the program textures on top of each other BEFORE recording it
    copy_buffer: ID3D11Texture2D,
    copy_buffer_tid: TextureId,
    show_window: bool,
    show_config_window: bool,
    recording: bool,
    encode_sender: Option<encoder_abstraction::Sender<encoder_abstraction::Dx11>>,
    encode_server: Option<std::thread::JoinHandle<()>>,
    start_time: Instant,
    frame_idx: usize,
    config: Config,
}

impl AppState {
    pub fn update_d3d11_texture(&mut self, d3d11_state: &D3D11State, _: &egui::Context) {
        self.target_states.retain(|program| {
            !program.shared_memory.loaded() | (program.shared_memory.ref_count() > 1)
        });
        for program in self.target_states.iter_mut() {
            // println!("{:?}", program.name);
            let header = &mut program.shared_memory;

            let rendering_api = header.api();

            // TODO: There needs to be a CPU path for DX7, 8, and 9
            let in_use_texture = if rendering_api.nt_handle_in_use() {
                &program.textures[1]
            } else {
                &program.textures[0]
            };

            if let Some(texture) = in_use_texture {
                unsafe {
                    d3d11_state.ctx.CopySubresourceRegion(
                        &*self.copy_buffer,
                        0,
                        0,
                        0,
                        0,
                        texture,
                        0,
                        None,
                    );
                }

                let encode_texture = create_texture(&d3d11_state.device, 1920, 1080, false, false).unwrap();
                unsafe {
                    d3d11_state.ctx.CopySubresourceRegion(
                        &encode_texture,
                        0,
                        0,
                        0,
                        0,
                        texture,
                        0,
                        None,
                    );
                }

                if let Some(sender) = &mut self.encode_sender {
                    if let Err(encoder_abstraction::EncSendError::InputFull) = sender.send(
                        encode_texture,
                        self.start_time.duration_since(Instant::now()).as_millis() as u64,
                        self.frame_idx,
                        1920 * 4,
                    ) {
                        println!("Frame dropped");
                    }
                    self.frame_idx += 1;
                }
            }
        }
    }

    pub fn update(&mut self, d3d11_state: &D3D11State, ctx: &egui::Context) {
        self.update_d3d11_texture(d3d11_state, ctx);

        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                // Flips texture, is cool but I'm not sure this is the right way to do this... : uv([Pos2::new(0.0, 1.0), Pos2::new(1.0, 0.0)])
                let image = Image::from_texture((self.copy_buffer_tid, [1920.0, 1080.0].into()))
                    .max_size([1920.0 - 500.0, 1080.0].into())
                    .shrink_to_fit();
                ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                    ui.add(image);
                });

                egui::panel::CentralPanel::default().show_inside(ui, |ui| {
                    ui.with_layout(
                        Layout::top_down(egui::Align::Min).with_main_wrap(true),
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    let response = ui.button("Add program");
                                    if response.clicked() {
                                        self.show_window = true;
                                    }
                                });
                                ui.vertical(|ui| {
                                    for program in &self.target_states {
                                        ui.label(program.name.to_string_lossy());
                                    }
                                });
                                ui.vertical(|ui| {
                                    let response = ui.button("Config");
                                    if response.clicked() {
                                        self.show_config_window = true;
                                    }

                                    if !self.recording {
                                        let response = ui.button("Start Recording");
                                        if response.clicked() {
                                            let desc =
                                                unsafe { d3d11_state.adapter.GetDesc().unwrap() };
                                            let vendor = match desc.VendorId {
                                                0x8086 => panic!("Intel is not yet supported"),
                                                0x10DE => encoder_abstraction::Vendor::Nvidia,
                                                0x1002 => encoder_abstraction::Vendor::AMD,
                                                unknown => panic!("Unknown vendor ID: {unknown}"),
                                            };
                                            let (sender, receiver) =
                                                encoder_abstraction::Sender::init(
                                                    vendor,
                                                    &d3d11_state.device,
                                                    self.config.clone(),
                                                );
                                            let file = std::fs::File::create("output.h264").unwrap();

                                            let handle = std::thread::spawn(move || {
                                                let mut file = file;
                                                loop {
                                                    if let Err(e) = receiver.recv(|slice| {
                                                        file.write_all(slice).unwrap();
                                                    }) {
                                                        match e {
                                                            encoder_abstraction::RecvError::Repeat => continue,
                                                            encoder_abstraction::RecvError::Eof => break,
                                                        }
                                                    }
                                                }
                                            });

                                            self.encode_sender = Some(sender);
                                            self.encode_server = Some(handle);
                                            self.start_time = Instant::now();
                                            self.frame_idx = 0;
                                            self.recording = true;
                                        }
                                    } else {
                                        let response = ui.button("Stop recording");
                                        if response.clicked() {
                                            self.recording = false;
                                            if let Some(sender) = self.encode_sender.take() && let Some(server) = self.encode_server.take() {
                                                sender.close();
                                                server.join().unwrap();
                                                self.recording = false;
                                            }
                                        }
                                    }
                                })
                            });
                        },
                    );

                    egui::Window::new("Config")
                        .open(&mut self.show_config_window)
                        .show(ctx, |ui| {
                            egui::ComboBox::from_label("Tuning Info").show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.config.tuning_info,
                                    encoder_abstraction::TuningInfo::LowLatency,
                                    "Low Latency",
                                );
                                ui.selectable_value(
                                    &mut self.config.tuning_info,
                                    encoder_abstraction::TuningInfo::UltraLowLatency,
                                    "Ultra Low Latency",
                                );
                            })
                        });

                    egui::Window::new("Add Program")
                        .open(&mut self.show_window)
                        .show(ctx, |ui| {
                            let mut windows = Vec::new();
                            self.system.refresh_all();
                            system_ext::System::new()
                                .unwrap()
                                .enum_windows(|name, process_id, window_id| {
                                    // println!("{name:?}");

                                    windows.push((name, process_id, window_id));
                                    true
                                })
                                .unwrap();

                            for (name, id, _window_id) in windows {
                                let watch = ui.button(name.to_string_lossy());

                                if watch.clicked() {
                                    // TODO: This code needs to be in the DLL?
                                    // let dc = unsafe { GetDC(Some(window_id)) };
                                    // let format = unsafe { GetPixelFormat(dc) };
                                    // let mut desc = PIXELFORMATDESCRIPTOR::default();
                                    // let err = unsafe { DescribePixelFormat(dc, format, size_of::<PIXELFORMATDESCRIPTOR>() as u32, Some(&raw mut desc)) };
                                    // if err == 0 {
                                    //     println!("{:?}", unsafe { GetLastError() });
                                    // }
                                    // println!("{desc:?}");

                                    let process = self.system.process(Pid::from_u32(id)).unwrap();
                                    let (shared_mem, textures) =
                                        inject(process, &d3d11_state.device).unwrap();
                                    self.target_states.push(TargetState {
                                        name,
                                        _pid: id,
                                        shared_memory: shared_mem,
                                        textures,
                                    });
                                }
                            }
                        })
                });
            });
    }
}

struct TargetState {
    name: OsString,
    // TODO: Figure out why this is here?
    _pid: u32,
    shared_memory: shmem::Shmem<shared_defs::SharedMemoryHeader>,
    textures: [Option<ID3D11Texture2D>; 2],
}

struct EguiState {
    winit: egui_winit::State,
    renderer: egui_directx11::Renderer,
    ctx: egui::Context,
}

struct D3D11State {
    ctx: ID3D11DeviceContext,
    render_target: Option<ID3D11RenderTargetView>,
    swap_chain: IDXGISwapChain,
    device: ID3D11Device,
    adapter: IDXGIAdapter,
}

impl ApplicationHandler for WinitState {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        pub const DEFAULT_SIZE: PhysicalSize<u32> = PhysicalSize::new(600, 600);

        if self.window.is_none() {
            let system = System::new_with_specifics(
                RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
            );
            let window_attributes = Window::default_attributes()
                .with_title("dx-11-test")
                .with_inner_size(DEFAULT_SIZE);
            let window = event_loop.create_window(window_attributes).unwrap();

            let PhysicalSize { width, height } = window.outer_size();
            let RawWindowHandle::Win32(win32_handle) = window.window_handle().unwrap().as_raw()
            else {
                panic!("Expected Win32 handle")
            };

            let (swap_chain, device, context, adapter) =
                dx11::create_device_and_swap_chain(width, height, &win32_handle);

            let egui_ctx = egui::Context::default();
            egui_ctx.set_theme(egui::Theme::Light);
            let mut egui_renderer = egui_directx11::Renderer::new(&device).unwrap();
            let egui_winit = egui_winit::State::new(
                egui_ctx.clone(),
                egui_ctx.viewport_id(),
                &window,
                None,
                None,
                None,
            );

            let render_target = render_target(&device, &swap_chain)
                .expect("Render target was created successfully");

            let new_texture = create_texture(&device, 1920, 1080, false, false)
                .expect("Texture was allocated successfully");

            let copy_buffer_tid = egui_renderer.register_native_texture(new_texture.clone());


            *self = Self {
                window: Some(window),
                egui_state: Some(EguiState {
                    ctx: egui_ctx,
                    winit: egui_winit,
                    renderer: egui_renderer,
                }),
                app_state: Some(AppState {
                    system,
                    target_states: Vec::new(),
                    copy_buffer: new_texture,
                    copy_buffer_tid,
                    show_window: false,
                    show_config_window: false,
                    // "Default" config
                    config: Config {
                        codec: encoder_abstraction::Codec::H264,
                        tuning_info: encoder_abstraction::TuningInfo::LowLatency,
                        format: encoder_abstraction::Format::RGBA,
                        target_bit_rate: 12_000_000,
                        resolution: [1920, 1080],
                        frame_rate: [60, 1],
                        gop_len: 600,
                    },
                    encode_sender: None,
                    encode_server: None,
                    recording: false,
                    frame_idx: 0,
                    start_time: Instant::now(),
                }),
                d3d11_state: Some(D3D11State {
                    ctx: context,
                    device,
                    render_target,
                    swap_chain,
                    adapter,
                }),
            };
        };
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let egui = self.egui_state.as_mut().unwrap();
        let window = self.window.as_mut().unwrap();
        let app_state = self.app_state.as_mut().unwrap();
        let d3d11_state = self.d3d11_state.as_mut().unwrap();

        let response = egui.winit.on_window_event(window, &event);

        if response.consumed {
            return;
        }

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
                        app_state.update(d3d11_state, ctx);
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
    }
}

/// This injects using load library injection, while there are alternatives, they are not the current goal of the project
fn inject(
    process: &Process,
    device: &ID3D11Device,
) -> Result<(Shmem<SharedMemoryHeader>, [Option<ID3D11Texture2D>; 2]), ()> {
    const SCREEN_RECORDER_32: &[u8] = include_bytes!("../../screen_recorder_32.dll");
    const SCREEN_RECORDER_64: &[u8] = include_bytes!("../../screen_recorder_64.dll");
    const SHARED_RIGHTS: u32 = DXGI_SHARED_RESOURCE_READ.0 | DXGI_SHARED_RESOURCE_WRITE.0;
    // TODO: Make sure that these are the only dlls that can be targetted
    const NT_HANDLE_APIS: &[&str] = &["d3d11.dll", "d3d12.dll", "opengl32.dll", "vulkan-1.dll"];
    const HANDLE_APIS: &[&str] = &["d3d9.dll", "d3d10.dll"];

    let name = format!("SylvScreenShare{}\0", process.pid().as_u32());
    let shared_memory: Shmem<SharedMemoryHeader> =
        shmem::Shmem::new(CStr::from_bytes_with_nul(name.as_bytes()).unwrap());

    let target_process = process_ext::Process::new(process);
    let mut textures = [None, None];
    let mut nt_handle = false;
    let mut handle = false;

    target_process
        .iter_modules(|ostr| {
            let dll_name = std::path::Path::new(ostr).file_name().unwrap();

            for &api in NT_HANDLE_APIS {
                if dll_name.to_ascii_lowercase() == api {
                    nt_handle |= true;
                    break;
                }
            }

            for &api in HANDLE_APIS {
                if dll_name == api {
                    handle |= true;
                    break;
                }
            }

            println!("{dll_name:?}");
        })
        .unwrap();

    let current_process = process_ext::Process::current_process();

    // TODO: Is there another way to represent this? A struct maybe?
    if handle {
        if let Some(texture) = create_texture(device, 1920, 1080, false, true) {
            let resource = texture.cast::<IDXGIResource>().unwrap();

            let handle = unsafe { resource.GetSharedHandle().unwrap() };
            shared_memory.set_shared_handle(handle.0);

            textures[0] = Some(texture);
        }
    };

    if nt_handle {
        if let Some(texture) = create_texture(device, 1920, 1080, true, true) {
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

            shared_memory.set_nt_shared_handle(dup_handle.0);

            textures[1] = Some(texture);
        }
    }

    let mut dll_path = std::env::current_dir().unwrap();

    if target_process.is_64_bit() {
        dll_path.push("screen_recorder_64.dll");
        std::fs::write(&dll_path, SCREEN_RECORDER_64).unwrap();
    } else {
        dll_path.push("screen_recorder_32.dll");
        std::fs::write(&dll_path, SCREEN_RECORDER_32).unwrap();
    }

    unsafe { target_process.load_remote_library(&dll_path) }.unwrap();

    Ok((shared_memory, textures))
}

fn create_texture(
    device: &ID3D11Device,
    width: u32,
    height: u32,
    nt_handle: bool,
    shared: bool,
) -> Option<ID3D11Texture2D> {
    let mut texture = None;
    unsafe {
        device.CreateTexture2D(
            &d3d11_texture_description(width, height, nt_handle, shared),
            None,
            Some(&mut texture),
        )
    }
    .unwrap();

    texture
}
