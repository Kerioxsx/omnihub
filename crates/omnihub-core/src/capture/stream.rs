//! Screen sharing from the PC to a phone browser.
//!
//! Capture uses DXGI Desktop Duplication on Windows (only changed frames,
//! straight from the GPU), with a GDI fallback. Frames are scaled with SIMD
//! and encoded as JPEG, then sent over a WebSocket. Pacing is driven by the
//! viewer: at most two frames are in flight, and whatever the screen shows
//! when the viewer catches up is what gets sent next, so slow links lose
//! frames instead of building up delay. Quality adapts to the measured
//! round trip.
//!
//! This is the zero-install path that works in any browser. For 4K60 with
//! hardware encoding, OmniHub hands off to Sunshine + Moonlight (see
//! `bridges`); true zero latency does not exist (capture + encode + network
//! + decode), but a LAN round trip here is typically 30–80 ms.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub label: String,
    pub max_width: u32,
    pub quality: u8,
    pub fps: u32,
}

pub fn presets() -> Vec<Preset> {
    let p = |id: &str, label: &str, max_width, quality, fps| Preset { id: id.into(), label: label.into(), max_width, quality, fps };
    vec![
        p("saver", "Data saver · 720p", 1280, 55, 30),
        p("balanced", "Balanced · 1080p", 1920, 70, 60),
        p("sharp", "Sharp · 1440p", 2560, 78, 60),
        p("max", "Maximum · up to 4K", 3840, 85, 60),
    ]
}

pub fn preset(id: &str) -> Preset {
    presets().into_iter().find(|p| p.id == id).unwrap_or_else(|| presets()[1].clone())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

/// A captured BGRA frame.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub data: Vec<u8>,
    /// Pointer position relative to the frame, if it is on this monitor.
    pub cursor: Option<(i32, i32)>,
}

// Not `Send`: a source is created and used on the capture thread only.
pub trait FrameSource {
    /// Wait up to `timeout` for a new frame. `Ok(false)` means nothing changed.
    fn next_frame(&mut self, timeout: Duration) -> std::io::Result<bool>;
    /// The most recent frame, if any was captured yet.
    fn last(&self) -> Option<&Frame>;
    fn monitor(&self) -> MonitorInfo;
}

/// Animated test pattern: used by the headless server and tests, and on
/// systems without a capturable display.
pub struct SyntheticSource {
    frame: Frame,
    started: Instant,
    last: Option<Instant>,
}

impl SyntheticSource {
    pub fn new(width: u32, height: u32) -> Self {
        SyntheticSource {
            frame: Frame { width, height, stride: width as usize * 4, data: vec![0; (width * height * 4) as usize], cursor: None },
            started: Instant::now(),
            last: None,
        }
    }
}

impl FrameSource for SyntheticSource {
    fn next_frame(&mut self, timeout: Duration) -> std::io::Result<bool> {
        // ~60 fps.
        let period = Duration::from_millis(16);
        if let Some(last) = self.last {
            let since = last.elapsed();
            if since < period {
                let wait = (period - since).min(timeout);
                std::thread::sleep(wait);
                if wait < period - since {
                    return Ok(false);
                }
            }
        }
        self.last = Some(Instant::now());
        let t = self.started.elapsed().as_secs_f32();
        let (w, h) = (self.frame.width as usize, self.frame.height as usize);
        let bx = ((t * 0.6).sin() * 0.5 + 0.5) * (w as f32 - 200.0);
        let by = ((t * 0.9).cos() * 0.5 + 0.5) * (h as f32 - 200.0);
        for y in 0..h {
            let row = &mut self.frame.data[y * w * 4..(y + 1) * w * 4];
            for x in 0..w {
                let inside = (x as f32) >= bx && (x as f32) < bx + 200.0 && (y as f32) >= by && (y as f32) < by + 200.0;
                let px = &mut row[x * 4..x * 4 + 4];
                if inside {
                    px.copy_from_slice(&[250, 250, 250, 255]);
                } else {
                    px[0] = (40 + (x * 120 / w)) as u8; // B
                    px[1] = (20 + (y * 60 / h)) as u8; // G
                    px[2] = (60 + ((t * 30.0) as usize + x / 8) % 120) as u8; // R
                    px[3] = 255;
                }
            }
        }
        self.frame.cursor = Some(((bx + 100.0) as i32, (by + 100.0) as i32));
        Ok(true)
    }

    fn last(&self) -> Option<&Frame> {
        self.last.map(|_| &self.frame)
    }

    fn monitor(&self) -> MonitorInfo {
        MonitorInfo { index: 0, name: "Test pattern".into(), x: 0, y: 0, width: self.frame.width, height: self.frame.height, primary: true }
    }
}

#[cfg(windows)]
pub mod dxgi {
    //! DXGI Desktop Duplication capture.
    use super::*;
    use windows::core::Interface;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ,
        D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    };
    use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, IDXGIOutput1, IDXGIOutputDuplication, IDXGIResource, DXGI_ERROR_ACCESS_LOST, DXGI_ERROR_WAIT_TIMEOUT, DXGI_OUTDUPL_FRAME_INFO};

    fn err(e: windows::core::Error) -> std::io::Error {
        std::io::Error::other(e.message())
    }

    pub struct DxgiSource {
        index: usize,
        device: ID3D11Device,
        context: ID3D11DeviceContext,
        dup: IDXGIOutputDuplication,
        staging: Option<ID3D11Texture2D>,
        info: MonitorInfo,
        frame: Frame,
        have_frame: bool,
    }

    /// The nth output across all adapters, with its adapter.
    fn find_output(index: usize) -> std::io::Result<(IDXGIAdapter1, IDXGIOutput1, MonitorInfo)> {
        unsafe {
            let factory: IDXGIFactory1 = CreateDXGIFactory1().map_err(err)?;
            let mut n = 0usize;
            let mut a = 0u32;
            while let Ok(adapter) = factory.EnumAdapters1(a) {
                let mut o = 0u32;
                while let Ok(output) = adapter.EnumOutputs(o) {
                    let desc = output.GetDesc().map_err(err)?;
                    if desc.AttachedToDesktop.as_bool() {
                        if n == index {
                            let r = desc.DesktopCoordinates;
                            let name = String::from_utf16_lossy(&desc.DeviceName).trim_end_matches('\0').to_string();
                            let info = MonitorInfo {
                                index,
                                name,
                                x: r.left,
                                y: r.top,
                                width: (r.right - r.left) as u32,
                                height: (r.bottom - r.top) as u32,
                                primary: r.left == 0 && r.top == 0,
                            };
                            return Ok((adapter, output.cast().map_err(err)?, info));
                        }
                        n += 1;
                    }
                    o += 1;
                }
                a += 1;
            }
        }
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "monitor not found"))
    }

    impl DxgiSource {
        pub fn new(index: usize) -> std::io::Result<Self> {
            let (adapter, output, info) = find_output(index)?;
            unsafe {
                let mut device = None;
                let mut context = None;
                D3D11CreateDevice(&adapter, D3D_DRIVER_TYPE_UNKNOWN, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut context)).map_err(err)?;
                let device = device.ok_or_else(|| std::io::Error::other("no D3D11 device"))?;
                let context = context.ok_or_else(|| std::io::Error::other("no D3D11 context"))?;
                let dup = output.DuplicateOutput(&device).map_err(err)?;
                let (w, h) = (info.width, info.height);
                Ok(DxgiSource {
                    index,
                    device,
                    context,
                    dup,
                    staging: None,
                    frame: Frame { width: w, height: h, stride: w as usize * 4, data: vec![0; (w * h * 4) as usize], cursor: None },
                    info,
                    have_frame: false,
                })
            }
        }

        fn reopen(&mut self) -> std::io::Result<()> {
            *self = DxgiSource::new(self.index)?;
            Ok(())
        }

        fn cursor(&self) -> Option<(i32, i32)> {
            use windows::Win32::Foundation::POINT;
            use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
            let mut p = POINT::default();
            unsafe { GetCursorPos(&mut p) }.ok()?;
            let (x, y) = (p.x - self.info.x, p.y - self.info.y);
            (x >= 0 && y >= 0 && (x as u32) < self.info.width && (y as u32) < self.info.height).then_some((x, y))
        }
    }

    impl FrameSource for DxgiSource {
        fn last(&self) -> Option<&Frame> {
            self.have_frame.then_some(&self.frame)
        }

        fn next_frame(&mut self, timeout: Duration) -> std::io::Result<bool> {
            unsafe {
                let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
                let mut res: Option<IDXGIResource> = None;
                match self.dup.AcquireNextFrame(timeout.as_millis() as u32, &mut info, &mut res) {
                    Ok(()) => {}
                    Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => {
                        // Nothing new; the pointer may still have moved.
                        if self.have_frame {
                            let c = self.cursor();
                            if c != self.frame.cursor {
                                self.frame.cursor = c;
                                return Ok(true);
                            }
                        }
                        return Ok(false);
                    }
                    Err(e) if e.code() == DXGI_ERROR_ACCESS_LOST => {
                        // Mode change, UAC prompt, lock screen: start over.
                        std::thread::sleep(Duration::from_millis(200));
                        self.reopen()?;
                        return Ok(false);
                    }
                    Err(e) => return Err(err(e)),
                }
                let result = (|| -> std::io::Result<bool> {
                    if info.LastPresentTime == 0 && self.have_frame {
                        // Only the pointer changed.
                        return Ok(false);
                    }
                    let tex: ID3D11Texture2D = res.as_ref().ok_or_else(|| std::io::Error::other("no frame"))?.cast().map_err(err)?;
                    let mut desc = D3D11_TEXTURE2D_DESC::default();
                    tex.GetDesc(&mut desc);
                    if self.staging.is_none() || desc.Width != self.frame.width || desc.Height != self.frame.height {
                        let sd = D3D11_TEXTURE2D_DESC {
                            Width: desc.Width,
                            Height: desc.Height,
                            MipLevels: 1,
                            ArraySize: 1,
                            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                            Usage: D3D11_USAGE_STAGING,
                            BindFlags: 0,
                            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                            MiscFlags: 0,
                        };
                        let mut staging = None;
                        self.device.CreateTexture2D(&sd, None, Some(&mut staging)).map_err(err)?;
                        self.staging = staging;
                        self.frame.width = desc.Width;
                        self.frame.height = desc.Height;
                        self.frame.stride = desc.Width as usize * 4;
                        self.frame.data.resize((desc.Width * desc.Height * 4) as usize, 0);
                    }
                    let staging = self.staging.as_ref().unwrap();
                    self.context.CopyResource(staging, &tex);
                    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                    self.context.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).map_err(err)?;
                    let pitch = mapped.RowPitch as usize;
                    let row = self.frame.stride;
                    let src = std::slice::from_raw_parts(mapped.pData as *const u8, pitch * self.frame.height as usize);
                    for y in 0..self.frame.height as usize {
                        self.frame.data[y * row..(y + 1) * row].copy_from_slice(&src[y * pitch..y * pitch + row]);
                    }
                    self.context.Unmap(staging, 0);
                    Ok(true)
                })();
                let _ = self.dup.ReleaseFrame();
                let changed = result?;
                let cursor = self.cursor();
                if !changed && cursor == self.frame.cursor {
                    return Ok(false);
                }
                self.frame.cursor = cursor;
                self.have_frame = true;
                Ok(true)
            }
        }

        fn monitor(&self) -> MonitorInfo {
            self.info.clone()
        }
    }
}

/// GDI capture through xcap, for when duplication is unavailable (some
/// remote desktop sessions and virtual GPUs).
#[cfg(windows)]
pub struct GdiSource {
    monitor: xcap::Monitor,
    info: MonitorInfo,
    frame: Frame,
    last: Option<Instant>,
    fps: u32,
}

#[cfg(windows)]
impl GdiSource {
    pub fn new(index: usize) -> std::io::Result<Self> {
        let monitors = xcap::Monitor::all().map_err(|e| std::io::Error::other(e.to_string()))?;
        let monitor = monitors.into_iter().nth(index).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "monitor not found"))?;
        let info = MonitorInfo {
            index,
            name: monitor.friendly_name().or_else(|_| monitor.name()).unwrap_or_default(),
            x: monitor.x().unwrap_or(0),
            y: monitor.y().unwrap_or(0),
            width: monitor.width().unwrap_or(0),
            height: monitor.height().unwrap_or(0),
            primary: monitor.is_primary().unwrap_or(false),
        };
        Ok(GdiSource { monitor, frame: Frame { width: info.width, height: info.height, stride: info.width as usize * 4, data: Vec::new(), cursor: None }, info, last: None, fps: 30 })
    }
}

#[cfg(windows)]
impl FrameSource for GdiSource {
    fn last(&self) -> Option<&Frame> {
        self.last.map(|_| &self.frame)
    }

    fn next_frame(&mut self, timeout: Duration) -> std::io::Result<bool> {
        let period = Duration::from_millis(1000 / self.fps as u64);
        if let Some(l) = self.last {
            if l.elapsed() < period {
                std::thread::sleep((period - l.elapsed()).min(timeout));
                if l.elapsed() < period {
                    return Ok(false);
                }
            }
        }
        self.last = Some(Instant::now());
        let img = self.monitor.capture_image().map_err(|e| std::io::Error::other(e.to_string()))?;
        let (w, h) = img.dimensions();
        let mut data = img.into_raw();
        for px in data.chunks_exact_mut(4) {
            px.swap(0, 2); // RGBA -> BGRA
        }
        self.frame = Frame { width: w, height: h, stride: w as usize * 4, data, cursor: None };
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut p = POINT::default();
        if unsafe { GetCursorPos(&mut p) }.is_ok() {
            let (x, y) = (p.x - self.info.x, p.y - self.info.y);
            if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
                self.frame.cursor = Some((x, y));
            }
        }
        Ok(true)
    }

    fn monitor(&self) -> MonitorInfo {
        self.info.clone()
    }
}

pub fn fake_screen() -> bool {
    std::env::var("OMNIHUB_FAKE_SCREEN").is_ok_and(|v| v == "1") || !cfg!(windows)
}

pub fn monitors() -> Vec<MonitorInfo> {
    if fake_screen() {
        return vec![SyntheticSource::new(1600, 900).monitor()];
    }
    #[cfg(windows)]
    {
        xcap::Monitor::all()
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(index, m)| MonitorInfo {
                index,
                name: m.friendly_name().or_else(|_| m.name()).unwrap_or_else(|_| format!("Display {}", index + 1)),
                x: m.x().unwrap_or(0),
                y: m.y().unwrap_or(0),
                width: m.width().unwrap_or(0),
                height: m.height().unwrap_or(0),
                primary: m.is_primary().unwrap_or(false),
            })
            .collect()
    }
    #[cfg(not(windows))]
    Vec::new()
}

pub fn open_source(index: usize) -> std::io::Result<Box<dyn FrameSource>> {
    if fake_screen() {
        return Ok(Box::new(SyntheticSource::new(1600, 900)));
    }
    #[cfg(windows)]
    {
        match dxgi::DxgiSource::new(index) {
            Ok(s) => Ok(Box::new(s)),
            Err(e) => {
                tracing::warn!("desktop duplication unavailable ({e}); using GDI capture");
                Ok(Box::new(GdiSource::new(index)?))
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = index;
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "screen capture is only implemented on Windows"))
    }
}

/// Scales BGRA frames and encodes them as JPEG.
pub struct JpegEncoder {
    resizer: fast_image_resize::Resizer,
    scaled: Vec<u8>,
    pub out: Vec<u8>,
}

impl Default for JpegEncoder {
    fn default() -> Self {
        JpegEncoder { resizer: fast_image_resize::Resizer::new(), scaled: Vec::new(), out: Vec::with_capacity(1 << 20) }
    }
}

impl JpegEncoder {
    /// Encode `frame` scaled to at most `max_width` (keeping aspect ratio).
    /// Returns the encoded size.
    pub fn encode(&mut self, frame: &Frame, max_width: u32, quality: u8) -> std::io::Result<(u32, u32)> {
        use fast_image_resize::images::{Image, ImageRef};
        use fast_image_resize::{PixelType, ResizeAlg, ResizeOptions};
        let (w, h) = (frame.width, frame.height);
        let (tw, th) = if w > max_width {
            (max_width & !1, ((h as u64 * max_width as u64 / w as u64) as u32).max(2) & !1)
        } else {
            (w, h)
        };
        let tight;
        let src_bytes: &[u8] = if frame.stride == w as usize * 4 {
            &frame.data[..(w * h * 4) as usize]
        } else {
            tight = frame.data.chunks(frame.stride).take(h as usize).flat_map(|r| &r[..w as usize * 4]).copied().collect::<Vec<u8>>();
            &tight
        };
        let pixels: &[u8] = if (tw, th) != (w, h) {
            let src = ImageRef::new(w, h, src_bytes, PixelType::U8x4).map_err(|e| std::io::Error::other(e.to_string()))?;
            self.scaled.resize((tw * th * 4) as usize, 0);
            let mut dst = Image::from_slice_u8(tw, th, &mut self.scaled, PixelType::U8x4).map_err(|e| std::io::Error::other(e.to_string()))?;
            let opts = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(fast_image_resize::FilterType::Bilinear));
            self.resizer.resize(&src, &mut dst, &opts).map_err(|e| std::io::Error::other(e.to_string()))?;
            &self.scaled
        } else {
            src_bytes
        };
        self.out.clear();
        let enc = jpeg_encoder::Encoder::new(&mut self.out, quality.clamp(10, 95));
        enc.encode(pixels, tw as u16, th as u16, jpeg_encoder::ColorType::Bgra).map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok((tw, th))
    }
}

pub const FRAME_HEADER: usize = 20;
pub const MSG_FRAME: u8 = 1;

/// Binary frame message: header (little endian) then the JPEG.
/// `[type u8][flags u8][seq u32][w u16][h u16][cx i16][cy i16][encode_ms u16][ts_ms u32]`
pub fn frame_message(seq: u32, w: u32, h: u32, cursor: Option<(i32, i32)>, encode_ms: u16, ts_ms: u32, jpeg: &[u8]) -> Vec<u8> {
    let mut m = Vec::with_capacity(FRAME_HEADER + jpeg.len());
    m.push(MSG_FRAME);
    m.push(if cursor.is_some() { 0b10 } else { 0 });
    m.extend_from_slice(&seq.to_le_bytes());
    m.extend_from_slice(&(w as u16).to_le_bytes());
    m.extend_from_slice(&(h as u16).to_le_bytes());
    let (cx, cy) = cursor.unwrap_or((-1, -1));
    m.extend_from_slice(&(cx.clamp(-1, i16::MAX as i32) as i16).to_le_bytes());
    m.extend_from_slice(&(cy.clamp(-1, i16::MAX as i32) as i16).to_le_bytes());
    m.extend_from_slice(&encode_ms.to_le_bytes());
    m.extend_from_slice(&ts_ms.to_le_bytes());
    m.extend_from_slice(jpeg);
    m
}

/// Live parameters a viewer can change mid-stream.
#[derive(Debug, Clone)]
pub struct StreamParams {
    pub max_width: u32,
    pub quality: u8,
    pub fps: u32,
    pub target_quality: u8,
    pub target_width: u32,
}

impl From<&Preset> for StreamParams {
    fn from(p: &Preset) -> Self {
        StreamParams { max_width: p.max_width, quality: p.quality, fps: p.fps, target_quality: p.quality, target_width: p.max_width }
    }
}

/// Shared state between the capture thread and the socket task.
pub struct StreamControl {
    pub stop: AtomicBool,
    pub in_flight: AtomicU32,
    pub params: Mutex<StreamParams>,
    pub epoch: Instant,
    pub rtt_ms: AtomicU32,
    pub encode_ms: AtomicU32,
    pub sent_frames: AtomicU32,
    pub sent_bytes: std::sync::atomic::AtomicU64,
    pub monitor: Mutex<Option<MonitorInfo>>,
}

impl StreamControl {
    pub fn new(p: &Preset) -> Arc<Self> {
        Arc::new(StreamControl {
            stop: AtomicBool::new(false),
            in_flight: AtomicU32::new(0),
            params: Mutex::new(p.into()),
            epoch: Instant::now(),
            rtt_ms: AtomicU32::new(0),
            encode_ms: AtomicU32::new(0),
            sent_frames: AtomicU32::new(0),
            sent_bytes: std::sync::atomic::AtomicU64::new(0),
            monitor: Mutex::new(None),
        })
    }

    pub fn now_ms(&self) -> u32 {
        self.epoch.elapsed().as_millis() as u32
    }

    /// Called for each viewer acknowledgement carrying the frame's timestamp.
    pub fn ack(&self, ts_ms: u32) {
        let _ = self.in_flight.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v.saturating_sub(1)));
        let rtt = self.now_ms().wrapping_sub(ts_ms);
        if rtt < 10_000 {
            // Exponential moving average.
            let prev = self.rtt_ms.load(Ordering::Relaxed);
            let next = if prev == 0 { rtt } else { (prev * 7 + rtt) / 8 };
            self.rtt_ms.store(next, Ordering::Relaxed);
        }
    }

    /// Nudge quality and size toward what the link sustains.
    pub fn adapt(&self) {
        let rtt = self.rtt_ms.load(Ordering::Relaxed);
        let mut p = self.params.lock();
        if rtt > 160 {
            p.quality = p.quality.saturating_sub(6).max(30);
            if rtt > 300 {
                p.max_width = ((p.max_width as f32 * 0.85) as u32).max(640);
            }
        } else if rtt < 70 {
            p.quality = (p.quality + 2).min(p.target_quality);
            if rtt < 45 && p.max_width < p.target_width {
                p.max_width = ((p.max_width as f32 * 1.1) as u32).min(p.target_width);
            }
        }
    }
}

/// Capture/encode loop. Sends complete messages through `tx` until stopped
/// or the receiver goes away.
pub fn run_capture(monitor: usize, ctl: Arc<StreamControl>, tx: tokio::sync::mpsc::Sender<Vec<u8>>) -> std::io::Result<()> {
    let mut source = open_source(monitor)?;
    *ctl.monitor.lock() = Some(source.monitor());
    let mut enc = JpegEncoder::default();
    let mut seq = 0u32;
    let mut last_sent = Instant::now() - Duration::from_secs(1);
    let mut last_adapt = Instant::now();
    let mut pending_change = true;
    let mut last_keepalive = Instant::now();
    while !ctl.stop.load(Ordering::Relaxed) && !tx.is_closed() {
        let params = ctl.params.lock().clone();
        let period = Duration::from_millis(1000 / params.fps.max(1) as u64);
        if source.next_frame(Duration::from_millis(30))? {
            pending_change = true;
        }
        // Static screens still get a frame every couple of seconds so the
        // viewer knows the connection is alive.
        if !pending_change && last_keepalive.elapsed() < Duration::from_secs(2) {
            continue;
        }
        // Back-pressure: never more than two frames on the wire.
        if ctl.in_flight.load(Ordering::Relaxed) >= 2 {
            if last_sent.elapsed() > Duration::from_secs(2) {
                // Lost acks (tab hidden, network blip): start over.
                ctl.in_flight.store(0, Ordering::Relaxed);
            }
            continue;
        }
        if last_sent.elapsed() < period {
            continue;
        }
        let Some(frame) = source.last() else { continue };
        let started = Instant::now();
        let (w, h) = enc.encode(frame, params.max_width, params.quality)?;
        let encode_ms = started.elapsed().as_millis().min(u16::MAX as u128) as u16;
        let scale = w as f32 / frame.width.max(1) as f32;
        let cursor = frame.cursor.map(|(x, y)| ((x as f32 * scale) as i32, (y as f32 * scale) as i32));
        let msg = frame_message(seq, w, h, cursor, encode_ms, ctl.now_ms(), &enc.out);
        ctl.encode_ms.store(encode_ms as u32, Ordering::Relaxed);
        ctl.sent_bytes.fetch_add(msg.len() as u64, Ordering::Relaxed);
        ctl.sent_frames.fetch_add(1, Ordering::Relaxed);
        ctl.in_flight.fetch_add(1, Ordering::Relaxed);
        if tx.blocking_send(msg).is_err() {
            break;
        }
        seq = seq.wrapping_add(1);
        last_sent = Instant::now();
        last_keepalive = last_sent;
        pending_change = false;
        if last_adapt.elapsed() > Duration::from_millis(700) {
            ctl.adapt();
            last_adapt = Instant::now();
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum ViewerMessage {
    Ack { seq: u32, ts: u32 },
    Preset { id: String },
    Pointer { x: f32, y: f32, kind: PointerKind, #[serde(default)] button: u8 },
    Wheel { dy: f32, #[serde(default)] dx: f32 },
    Key { key: String },
    Text { text: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PointerKind {
    Move,
    Down,
    Up,
    Click,
}

/// Remote input (Windows only; needs the "allow control" setting).
pub mod input {
    use super::*;

    #[cfg(windows)]
    fn send(inputs: &[windows::Win32::UI::Input::KeyboardAndMouse::INPUT]) {
        use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT};
        unsafe {
            SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }

    #[cfg(windows)]
    fn mouse(dx: i32, dy: i32, data: i32, flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
        use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, INPUT_0, INPUT_MOUSE, MOUSEINPUT};
        INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData: data as u32, dwFlags: flags, time: 0, dwExtraInfo: 0 } } }
    }

    #[cfg(windows)]
    fn key(vk: u16, scan: u16, up: bool, unicode: bool) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
        use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY};
        let mut flags = KEYBD_EVENT_FLAGS(0);
        if up {
            flags |= KEYEVENTF_KEYUP;
        }
        if unicode {
            flags |= KEYEVENTF_UNICODE;
        }
        INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 } } }
    }

    /// Virtual-key code for a key name such as "Enter", "F5", "a".
    pub fn vk_for(name: &str) -> Option<u16> {
        let n = name.to_lowercase();
        Some(match n.as_str() {
            "enter" | "return" => 0x0D,
            "backspace" => 0x08,
            "tab" => 0x09,
            "escape" | "esc" => 0x1B,
            "space" => 0x20,
            "pageup" => 0x21,
            "pagedown" => 0x22,
            "end" => 0x23,
            "home" => 0x24,
            "left" | "arrowleft" => 0x25,
            "up" | "arrowup" => 0x26,
            "right" | "arrowright" => 0x27,
            "down" | "arrowdown" => 0x28,
            "insert" => 0x2D,
            "delete" | "del" => 0x2E,
            "win" | "meta" | "windows" => 0x5B,
            "ctrl" | "control" => 0x11,
            "alt" => 0x12,
            "shift" => 0x10,
            "printscreen" => 0x2C,
            "volumeup" => 0xAF,
            "volumedown" => 0xAE,
            "volumemute" => 0xAD,
            "mediaplaypause" | "playpause" => 0xB3,
            "medianext" => 0xB0,
            "mediaprev" => 0xB1,
            _ => {
                if let Some(f) = n.strip_prefix('f').and_then(|d| d.parse::<u16>().ok()).filter(|d| (1..=24).contains(d)) {
                    0x70 + f - 1
                } else if n.len() == 1 {
                    let c = n.chars().next()?.to_ascii_uppercase();
                    if c.is_ascii_alphanumeric() { c as u16 } else { return None }
                } else {
                    return None;
                }
            }
        })
    }

    /// Press a key or a chord like "Ctrl+Shift+Esc" / "Win+D".
    pub fn chord(spec: &str) -> bool {
        let keys: Vec<u16> = spec.split('+').filter_map(|k| vk_for(k.trim())).collect();
        if keys.is_empty() || keys.len() != spec.split('+').count() {
            return false;
        }
        #[cfg(windows)]
        {
            let mut seq: Vec<_> = keys.iter().map(|&k| key(k, 0, false, false)).collect();
            seq.extend(keys.iter().rev().map(|&k| key(k, 0, true, false)));
            send(&seq);
        }
        true
    }

    pub fn text(t: &str) {
        #[cfg(windows)]
        {
            let mut seq = Vec::new();
            for unit in t.encode_utf16().take(2000) {
                if unit == '\n' as u16 {
                    seq.push(key(0x0D, 0, false, false));
                    seq.push(key(0x0D, 0, true, false));
                } else {
                    seq.push(key(0, unit, false, true));
                    seq.push(key(0, unit, true, true));
                }
            }
            send(&seq);
        }
        #[cfg(not(windows))]
        let _ = t;
    }

    /// Map a normalised position on `m` to absolute virtual-desktop units.
    pub fn absolute(m: &MonitorInfo, x: f32, y: f32, virt: (i32, i32, i32, i32)) -> (i32, i32) {
        let (vx, vy, vw, vh) = virt;
        let px = m.x as f32 + x.clamp(0.0, 1.0) * (m.width.saturating_sub(1)) as f32;
        let py = m.y as f32 + y.clamp(0.0, 1.0) * (m.height.saturating_sub(1)) as f32;
        let ax = ((px - vx as f32) * 65535.0 / (vw - 1).max(1) as f32).round() as i32;
        let ay = ((py - vy as f32) * 65535.0 / (vh - 1).max(1) as f32).round() as i32;
        (ax, ay)
    }

    pub fn pointer(m: &MonitorInfo, x: f32, y: f32, kind: PointerKind, button: u8) {
        #[cfg(windows)]
        {
            use windows::Win32::UI::Input::KeyboardAndMouse::*;
            use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN};
            let virt = unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN), GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN)) };
            let (ax, ay) = absolute(m, x, y, virt);
            let base = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK;
            let (down, up) = match button {
                1 => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
                2 => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
                _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
            };
            let seq: Vec<INPUT> = match kind {
                PointerKind::Move => vec![mouse(ax, ay, 0, base)],
                PointerKind::Down => vec![mouse(ax, ay, 0, base | down)],
                PointerKind::Up => vec![mouse(ax, ay, 0, base | up)],
                PointerKind::Click => vec![mouse(ax, ay, 0, base), mouse(ax, ay, 0, base | down), mouse(ax, ay, 0, base | up)],
            };
            send(&seq);
        }
        #[cfg(not(windows))]
        let _ = (m, x, y, kind, button);
    }

    pub fn wheel(dy: f32, dx: f32) {
        #[cfg(windows)]
        {
            use windows::Win32::UI::Input::KeyboardAndMouse::{MOUSEEVENTF_HWHEEL, MOUSEEVENTF_WHEEL};
            let mut seq = Vec::new();
            if dy != 0.0 {
                seq.push(mouse(0, 0, (-dy * 1.2).round() as i32, MOUSEEVENTF_WHEEL));
            }
            if dx != 0.0 {
                seq.push(mouse(0, 0, (dx * 1.2).round() as i32, MOUSEEVENTF_HWHEEL));
            }
            send(&seq);
        }
        #[cfg(not(windows))]
        let _ = (dy, dx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_scales_and_encodes() {
        let mut src = SyntheticSource::new(1600, 900);
        assert!(src.next_frame(Duration::from_millis(50)).unwrap());
        let frame = src.last().unwrap();
        let mut enc = JpegEncoder::default();
        let (w, h) = enc.encode(frame, 1280, 70).unwrap();
        assert_eq!((w, h), (1280, 720));
        let img = image::load_from_memory(&enc.out).unwrap();
        assert_eq!((img.width(), img.height()), (1280, 720));
        let (w, _) = enc.encode(frame, 4000, 70).unwrap();
        assert_eq!(w, 1600, "never upscales");
    }

    #[test]
    fn frame_header_layout() {
        let m = frame_message(7, 1280, 720, Some((10, 20)), 5, 123456, b"JPEG");
        assert_eq!(m.len(), FRAME_HEADER + 4);
        assert_eq!(m[0], MSG_FRAME);
        assert_eq!(u32::from_le_bytes(m[2..6].try_into().unwrap()), 7);
        assert_eq!(u16::from_le_bytes(m[6..8].try_into().unwrap()), 1280);
        assert_eq!(i16::from_le_bytes(m[10..12].try_into().unwrap()), 10);
        assert_eq!(u32::from_le_bytes(m[16..20].try_into().unwrap()), 123456);
        assert_eq!(&m[20..], b"JPEG");
    }

    #[test]
    fn capture_loop_respects_backpressure() {
        let ctl = StreamControl::new(&preset("saver"));
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let c2 = ctl.clone();
        let th = std::thread::spawn(move || run_capture(0, c2, tx));
        let mut got = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(3);
        while got.len() < 2 && Instant::now() < deadline {
            if let Ok(m) = rx.try_recv() {
                got.push(m);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(got.len(), 2);
        // Without acks nothing more is sent.
        std::thread::sleep(Duration::from_millis(300));
        assert!(rx.try_recv().is_err());
        let ts = u32::from_le_bytes(got[0][16..20].try_into().unwrap());
        ctl.ack(ts);
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut more = None;
        while more.is_none() && Instant::now() < deadline {
            more = rx.try_recv().ok();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(more.is_some(), "an ack frees a slot");
        ctl.stop.store(true, Ordering::Relaxed);
        drop(rx);
        th.join().unwrap().unwrap();
    }

    #[test]
    fn viewer_messages_parse() {
        let m: ViewerMessage = serde_json::from_str(r#"{"t":"pointer","x":0.5,"y":0.25,"kind":"click"}"#).unwrap();
        assert_eq!(m, ViewerMessage::Pointer { x: 0.5, y: 0.25, kind: PointerKind::Click, button: 0 });
        let m: ViewerMessage = serde_json::from_str(r#"{"t":"ack","seq":3,"ts":99}"#).unwrap();
        assert_eq!(m, ViewerMessage::Ack { seq: 3, ts: 99 });
        assert_eq!(input::vk_for("F5"), Some(0x74));
        assert_eq!(input::vk_for("a"), Some(0x41));
        assert_eq!(input::vk_for("nope"), None);
        let m = MonitorInfo { index: 0, name: String::new(), x: 1920, y: 0, width: 1920, height: 1080, primary: false };
        assert_eq!(input::absolute(&m, 0.0, 0.0, (0, 0, 3840, 1080)), (32776, 0));
        assert_eq!(input::absolute(&m, 1.0, 1.0, (0, 0, 3840, 1080)), (65535, 65535));
    }
}
