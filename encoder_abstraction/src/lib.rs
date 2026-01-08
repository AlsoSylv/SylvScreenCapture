use std::{
    marker::PhantomData,
    sync::mpsc::{Receiver, SyncSender, TryRecvError},
};

use amffi::{
    amf_init,
    components::{
        component::AMFComponent,
        video_encoder_vce::{
            AMF_VIDEO_ENCODER_FRAMERATE, AMF_VIDEO_ENCODER_FRAMESIZE, AMF_VIDEO_ENCODER_IDR_PERIOD,
            AMF_VIDEO_ENCODER_TARGET_BITRATE, AMF_VIDEO_ENCODER_USAGE, AMFVideoEncoderUsage,
        },
    },
    core::{
        buffer::AMFBuffer,
        context::AMFContext,
        interface::Interface,
        platform::{AMFRate, AMFSize},
        result::AMFError,
    },
};
use nvenc::{
    bitstream::BitStream,
    encoder::{Encoder, RegisteredResource},
    session::Session,
    sys::{
        guids::{NV_ENC_CODEC_H264_GUID, NV_ENC_PRESET_P3_GUID},
        result::NVencError,
    },
};
use widestring::WideCStr;
use windows::Win32::Graphics::Direct3D11::ID3D11Device;
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;

pub trait Api {
    type Texture;
    type Device;
}

pub struct Dx11;

impl Api for Dx11 {
    type Texture = ID3D11Texture2D;
    type Device = ID3D11Device;
}

pub enum Vendor {
    AMD,
    Nvidia,
}

enum VendorInternal {
    Amd(AMFContext, AMFComponent),
    Nvidia(Encoder, SyncSender<NvMessage>, Receiver<BitStream>),
}

enum NvMessage {
    Cnt(BitStream, RegisteredResource),
    End,
}

unsafe impl Send for NvMessage {}

pub struct Sender<A: Api> {
    format: Format,
    vendor: VendorInternal,
    marker: PhantomData<A>,
}

enum VendorInternalRecv {
    Amd(AMFComponent),
    Nvidia(Receiver<NvMessage>, SyncSender<BitStream>),
}

pub struct EncoderReceiver {
    vendor: VendorInternalRecv,
}

pub enum RecvError {
    Repeat,
    Eof,
}

impl EncoderReceiver {
    pub fn recv<F>(&self, mut f: F) -> Result<(), RecvError>
    where
        F: FnMut(&[u8]),
    {
        match &self.vendor {
            VendorInternalRecv::Amd(enc) => {
                match enc.query_output() {
                    Ok(data) => {
                        let buffer: AMFBuffer = data.cast().unwrap();
                        f(unsafe {
                            std::slice::from_raw_parts(
                                buffer.get_native() as _,
                                buffer.get_size() as usize,
                            )
                        });
                    }
                    Err(e) => {
                        return match e {
                            AMFError::Repeat => Err(RecvError::Repeat),
                            AMFError::Eof => Err(RecvError::Eof),
                            e => panic!("Unknown Error {e:?}"),
                        };
                    }
                }
                Ok(())
            }
            VendorInternalRecv::Nvidia(rx, tx) => {
                match rx.try_recv() {
                    Ok(NvMessage::End) => Err(RecvError::Eof),
                    Ok(NvMessage::Cnt(buf, _reg)) => {
                        {
                            match buf.try_lock(true) {
                                Ok(lock) => {
                                    f(lock.as_slice());
                                },
                                Err(NVencError::LockBusy) => return Err(RecvError::Repeat),
                                Err(e) => panic!("Unknown Error: {e:?}"),
                            }
                        }
                        if tx.send(buf).is_err() {
                            // This indicates the other half was dropped, safe as a recv error
                            Err(RecvError::Eof)
                        } else {
                            Ok(())
                        }
                    }
                    Err(TryRecvError::Empty) => Err(RecvError::Repeat),
                    Err(TryRecvError::Disconnected) => Err(RecvError::Eof),
                }
            }
        }
    }
}

pub enum EncSendError {
    InputFull,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TuningInfo {
    LowLatency,
    UltraLowLatency,
}

impl TuningInfo {
    fn amd(&self) -> amffi::components::video_encoder_vce::AMFVideoEncoderUsage {
        match self {
            Self::LowLatency => AMFVideoEncoderUsage::LowLatency,
            Self::UltraLowLatency => AMFVideoEncoderUsage::UltraLowLatency,
        }
    }

    fn nv(&self) -> nvenc::sys::enums::NVencTuningInfo {
        match self {
            Self::LowLatency => nvenc::sys::enums::NVencTuningInfo::LowLatency,
            Self::UltraLowLatency => nvenc::sys::enums::NVencTuningInfo::UltraLowLatency,
        }
    }
}

#[derive(Clone)]
pub enum Format {
    RGBA,
}

impl Format {
    fn amd(&self) -> amffi::core::surface::AMFSurfaceFormat {
        match self {
            Self::RGBA => amffi::core::surface::AMFSurfaceFormat::RGBA,
        }
    }

    fn nv(&self) -> nvenc::sys::enums::NVencBufferFormat {
        match self {
            Self::RGBA => nvenc::sys::enums::NVencBufferFormat::ARGB,
        }
    }
}

#[derive(Clone)]
pub enum Codec {
    H264,
}

impl Codec {
    fn amd(&self) -> &WideCStr {
        match self {
            Self::H264 => amffi::components::video_encoder_vce::AMF_VIDEO_ENCODER_VCE_AVC,
        }
    }

    fn nv(&self) -> nvenc::sys::structs::Guid {
        match self {
            Self::H264 => NV_ENC_CODEC_H264_GUID,
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub codec: Codec,
    pub tuning_info: TuningInfo,
    pub format: Format,
    pub target_bit_rate: u32,
    pub resolution: [u32; 2],
    pub frame_rate: [u32; 2],
    pub gop_len: u16,
}

impl Sender<Dx11> {
    pub fn init(
        vendor: Vendor,
        device: &<Dx11 as Api>::Device,
        config: Config,
    ) -> (Self, EncoderReceiver) {
        match vendor {
            Vendor::AMD => {
                let lib = amf_init().unwrap();
                let factory = lib.init_factory(amffi::core::version::AMF_VERSION).unwrap();
                let context = factory.create_context().unwrap();
                context
                    .init_dx11(device.clone(), amffi::core::data::AMFDXVersion::DX11_0)
                    .unwrap(); // Create H264 init
                let component = factory
                    .create_component(&context, config.codec.amd())
                    .unwrap();
                component
                    .set_property(AMF_VIDEO_ENCODER_USAGE, config.tuning_info.amd() as i64)
                    .unwrap();
                component
                    .set_property(
                        AMF_VIDEO_ENCODER_TARGET_BITRATE,
                        config.target_bit_rate as i64,
                    )
                    .unwrap();

                component
                    .set_property(AMF_VIDEO_ENCODER_IDR_PERIOD, config.gop_len as i64)
                    .unwrap();
                let size = AMFSize::new(config.resolution[0] as i32, config.resolution[1] as i32);
                component
                    .set_property(AMF_VIDEO_ENCODER_FRAMESIZE, size)
                    .unwrap();
                let rate = AMFRate::new(config.frame_rate[0], config.frame_rate[1]);
                component
                    .set_property(AMF_VIDEO_ENCODER_FRAMERATE, rate)
                    .unwrap();
                component
                    .init(
                        config.format.amd(),
                        config.resolution[0] as i32,
                        config.resolution[1] as i32,
                    )
                    .unwrap();
                (
                    Sender {
                        vendor: VendorInternal::Amd(context, component.clone()),
                        format: config.format,
                        marker: PhantomData,
                    },
                    EncoderReceiver {
                        vendor: VendorInternalRecv::Amd(component),
                    },
                )
            }
            Vendor::Nvidia => {
                let session = Session::open_dx(device).unwrap();
                let (session, mut nv_config) = session
                    .get_encode_preset_config_ex(
                        config.codec.nv(),
                        NV_ENC_PRESET_P3_GUID,
                        config.tuning_info.nv(),
                    )
                    .unwrap();
                nv_config.preset_cfg.rc_params.rate_control_mode =
                    nvenc::sys::enums::NVencParamsRcMode::VBR;
                nv_config.preset_cfg.rc_params.average_bit_rate = config.target_bit_rate;
                nv_config.preset_cfg.gop_len = config.gop_len as u32;
                nv_config.preset_cfg.frame_interval_p = 1;
                let init_params = nvenc::session::InitParams {
                    encode_guid: config.codec.nv(),
                    preset_guid: NV_ENC_PRESET_P3_GUID,
                    resolution: config.resolution,
                    aspect_ratio: config.resolution,
                    frame_rate: config.frame_rate,
                    tuning_info: config.tuning_info.nv(),
                    buffer_format: config.format.nv(),
                    encode_config: &mut nv_config.preset_cfg,
                    enable_ptd: true,
                    max_encoder_resolution: [0, 0],
                };

                let encoder = session.init_encoder(init_params).unwrap();
                let (processed, to_use) = std::sync::mpsc::sync_channel::<BitStream>(2);
                let (re_use, to_process) = std::sync::mpsc::sync_channel::<NvMessage>(2);
                processed
                    .send(encoder.create_bitstream_buffer().unwrap())
                    .unwrap();
                processed
                    .send(encoder.create_bitstream_buffer().unwrap())
                    .unwrap();

                (
                    Sender {
                        vendor: VendorInternal::Nvidia(encoder, re_use, to_use),
                        format: config.format,
                        marker: PhantomData,
                    },
                    EncoderReceiver {
                        vendor: VendorInternalRecv::Nvidia(to_process, processed),
                    },
                )
            }
        }
    }

    pub fn send(
        &mut self,
        texture: <Dx11 as Api>::Texture,
        timestamp: u64,
        frame_idx: usize,
        pitch: u32,
    ) -> Result<(), EncSendError> {
        match &self.vendor {
            VendorInternal::Amd(ctx, enc) => {
                let surface = ctx.create_surface_from_dx11_native(&texture).unwrap();
                surface
                    .set_property(widestring::widecstr!("StartTimeProperty"), timestamp as i64)
                    .unwrap();
                match enc.submit_input(&surface) {
                    Ok(()) => Ok(()),
                    Err(AMFError::InputFull) => Err(EncSendError::InputFull),
                    Err(e) => panic!("Unknown Error {e:?}"),
                }
            }
            VendorInternal::Nvidia(enc, tx, rx) => {
                let out = rx.recv().unwrap();
                let registered = enc
                    .register_resource_dx11(&texture, self.format.nv(), pitch)
                    .unwrap();
                enc.encode_picture(
                    &registered,
                    &out,
                    frame_idx,
                    timestamp,
                    nvenc::sys::enums::NVencBufferFormat::ABGR,
                    nvenc::sys::enums::NVencPicStruct::Frame,
                    nvenc::sys::enums::NVencPicType::P,
                    None,
                )
                .unwrap();
                match tx.send(NvMessage::Cnt(out, registered)) {
                    Ok(()) => Ok(()),
                    Err(e) => panic!("{e}"),
                }
            }
        }
    }
}

impl<A: Api> Sender<A> {
    pub fn close(&self) {
        match &self.vendor {
            VendorInternal::Amd(_, enc) => {
                enc.drain().unwrap();
            }
            VendorInternal::Nvidia(enc, tx, _) => {
                enc.end_encode().unwrap();
                let _ = tx.send(NvMessage::End);
            }
        }
    }
}

impl<A: Api> Drop for Sender<A> {
    fn drop(&mut self) {
        self.close();
    }
}
