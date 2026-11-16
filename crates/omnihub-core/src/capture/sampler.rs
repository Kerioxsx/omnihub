//! Small colour grids of a monitor, for lights that follow the screen. On
//! Windows the graphics card shrinks each desktop frame (Desktop
//! Duplication, then mipmaps), so only a few kilobytes reach the processor
//! however large the screen is. Nothing is kept or sent anywhere: a grid
//! is overwritten by the next one.

use std::time::Duration;

use super::stream::{Frame, FrameSource, MonitorInfo};

/// A monitor shrunk to a few dozen cells a side, RGB.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[u8; 3]>,
}

impl Grid {
    pub fn at(&self, x: usize, y: usize) -> [u8; 3] {
        self.px[y.min(self.h - 1) * self.w + x.min(self.w - 1)]
    }
}

pub trait Sampler {
    /// Wait up to `timeout` for the screen to change. `Ok(false)`: nothing new.
    fn next(&mut self, timeout: Duration) -> std::io::Result<bool>;
    fn grid(&self) -> Option<&Grid>;
    fn monitor(&self) -> MonitorInfo;
}

/// Shrink a BGRA frame to at most `max_w` cells across, averaging a few
/// pixels in each cell (enough for colour, cheap on the processor).
pub fn shrink(frame: &Frame, max_w: usize) -> Grid {
    let (fw, fh) = (frame.width.max(1) as usize, frame.height.max(1) as usize);
    let w = max_w.clamp(1, fw);
    let h = ((fh * w) / fw).max(1);
    let mut px = Vec::with_capacity(w * h);
    for gy in 0..h {
        for gx in 0..w {
            let (x0, x1) = (gx * fw / w, ((gx + 1) * fw / w).max(gx * fw / w + 1));
            let (y0, y1) = (gy * fh / h, ((gy + 1) * fh / h).max(gy * fh / h + 1));
            let (sx, sy) = (((x1 - x0) / 4).max(1), ((y1 - y0) / 4).max(1));
            let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
            let mut y = y0;
            while y < y1 {
                let mut x = x0;
                while x < x1 {
                    let i = y * frame.stride + x * 4;
                    if let Some(p) = frame.data.get(i..i + 3) {
                        b += p[0] as u32;
                        g += p[1] as u32;
                        r += p[2] as u32;
                        n += 1;
                    }
                    x += sx;
                }
                y += sy;
            }
            let n = n.max(1);
            px.push([(r / n) as u8, (g / n) as u8, (b / n) as u8]);
        }
    }
    Grid { w, h, px }
}

/// Grids from any frame source (the test pattern, GDI capture).
pub struct FrameSampler {
    src: Box<dyn FrameSource>,
    grid: Option<Grid>,
}

impl FrameSampler {
    pub fn new(src: Box<dyn FrameSource>) -> Self {
        FrameSampler { src, grid: None }
    }
}

impl Sampler for FrameSampler {
    fn next(&mut self, timeout: Duration) -> std::io::Result<bool> {
        if !self.src.next_frame(timeout)? {
            return Ok(false);
        }
        self.grid = self.src.last().map(|f| shrink(f, 64));
        Ok(self.grid.is_some())
    }

    fn grid(&self) -> Option<&Grid> {
        self.grid.as_ref()
    }

    fn monitor(&self) -> MonitorInfo {
        self.src.monitor()
    }
}

/// The monitor `index` (as `stream::monitors` lists them).
pub fn open(index: usize) -> std::io::Result<Box<dyn Sampler>> {
    if super::stream::fake_screen() {
        return Ok(Box::new(FrameSampler::new(Box::new(super::stream::SyntheticSource::new(320, 180)))));
    }
    #[cfg(windows)]
    {
        match dxgi::DxgiSampler::new(index) {
            Ok(s) => Ok(Box::new(s)),
            Err(e) => {
                tracing::warn!("screen sync: desktop duplication unavailable ({e}); using GDI capture");
                Ok(Box::new(FrameSampler::new(Box::new(super::stream::GdiSource::new(index)?))))
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = index;
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "screen capture is only implemented on Windows"))
    }
}

#[cfg(windows)]
mod dxgi {
    //! Desktop Duplication with the shrinking done by the GPU: each frame is
    //! copied into a texture with mipmaps, the GPU generates them, and only
    //! the level about 100 pixels wide is read back.
    use super::*;
    use windows::core::Interface;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11ShaderResourceView, ID3D11Texture2D, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_RESOURCE_MISC_GENERATE_MIPS, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
    };
    use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
    use windows::Win32::Graphics::Dxgi::{IDXGIOutputDuplication, IDXGIResource, DXGI_ERROR_ACCESS_LOST, DXGI_ERROR_WAIT_TIMEOUT, DXGI_OUTDUPL_FRAME_INFO};

    fn err(e: windows::core::Error) -> std::io::Error {
        std::io::Error::other(e.message())
    }

    /// The textures for one frame size.
    struct Chain {
        width: u32,
        height: u32,
        mipped: ID3D11Texture2D,
        view: ID3D11ShaderResourceView,
        /// The level read back, and its size.
        level: u32,
        small: ID3D11Texture2D,
        sw: u32,
        sh: u32,
    }

    pub struct DxgiSampler {
        index: usize,
        device: ID3D11Device,
        context: ID3D11DeviceContext,
        dup: IDXGIOutputDuplication,
        chain: Option<Chain>,
        info: MonitorInfo,
        grid: Option<Grid>,
    }

    impl DxgiSampler {
        pub fn new(index: usize) -> std::io::Result<Self> {
            let (adapter, output, info) = crate::capture::stream::dxgi::find_output(index)?;
            unsafe {
                let mut device = None;
                let mut context = None;
                D3D11CreateDevice(&adapter, D3D_DRIVER_TYPE_UNKNOWN, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut context)).map_err(err)?;
                let device = device.ok_or_else(|| std::io::Error::other("no D3D11 device"))?;
                let context = context.ok_or_else(|| std::io::Error::other("no D3D11 context"))?;
                let dup = output.DuplicateOutput(&device).map_err(err)?;
                Ok(DxgiSampler { index, device, context, dup, chain: None, info, grid: None })
            }
        }

        fn chain(&self, width: u32, height: u32) -> std::io::Result<Chain> {
            // The first level no wider than 128 pixels.
            let mut level = 0;
            while (width >> level) > 128 && level < 12 {
                level += 1;
            }
            let (sw, sh) = ((width >> level).max(1), (height >> level).max(1));
            unsafe {
                let md = D3D11_TEXTURE2D_DESC {
                    Width: width,
                    Height: height,
                    MipLevels: level + 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
                    CPUAccessFlags: 0,
                    MiscFlags: D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
                };
                let mut mipped = None;
                self.device.CreateTexture2D(&md, None, Some(&mut mipped)).map_err(err)?;
                let mipped = mipped.ok_or_else(|| std::io::Error::other("no texture"))?;
                let mut view = None;
                self.device.CreateShaderResourceView(&mipped, None, Some(&mut view)).map_err(err)?;
                let view = view.ok_or_else(|| std::io::Error::other("no view"))?;
                let sd = D3D11_TEXTURE2D_DESC {
                    Width: sw,
                    Height: sh,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Usage: D3D11_USAGE_STAGING,
                    BindFlags: 0,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                    MiscFlags: 0,
                };
                let mut small = None;
                self.device.CreateTexture2D(&sd, None, Some(&mut small)).map_err(err)?;
                let small = small.ok_or_else(|| std::io::Error::other("no texture"))?;
                Ok(Chain { width, height, mipped, view, level, small, sw, sh })
            }
        }

        fn read(&mut self, tex: &ID3D11Texture2D) -> std::io::Result<()> {
            unsafe {
                let mut desc = D3D11_TEXTURE2D_DESC::default();
                tex.GetDesc(&mut desc);
                if self.chain.as_ref().is_none_or(|c| c.width != desc.Width || c.height != desc.Height) {
                    self.chain = Some(self.chain(desc.Width, desc.Height)?);
                }
                let c = self.chain.as_ref().unwrap();
                self.context.CopySubresourceRegion(&c.mipped, 0, 0, 0, 0, tex, 0, None);
                self.context.GenerateMips(&c.view);
                self.context.CopySubresourceRegion(&c.small, 0, 0, 0, 0, &c.mipped, c.level, None);
                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                self.context.Map(&c.small, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).map_err(err)?;
                let pitch = mapped.RowPitch as usize;
                let (w, h) = (c.sw as usize, c.sh as usize);
                let src = std::slice::from_raw_parts(mapped.pData as *const u8, pitch * h);
                let mut px = Vec::with_capacity(w * h);
                for y in 0..h {
                    for x in 0..w {
                        let p = &src[y * pitch + x * 4..y * pitch + x * 4 + 3];
                        px.push([p[2], p[1], p[0]]);
                    }
                }
                self.context.Unmap(&c.small, 0);
                self.grid = Some(Grid { w, h, px });
                Ok(())
            }
        }
    }

    impl Sampler for DxgiSampler {
        fn next(&mut self, timeout: Duration) -> std::io::Result<bool> {
            unsafe {
                let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
                let mut res: Option<IDXGIResource> = None;
                match self.dup.AcquireNextFrame(timeout.as_millis() as u32, &mut info, &mut res) {
                    Ok(()) => {}
                    Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => return Ok(false),
                    Err(e) if e.code() == DXGI_ERROR_ACCESS_LOST => {
                        // Mode change, UAC prompt, lock screen: start over.
                        std::thread::sleep(Duration::from_millis(200));
                        *self = DxgiSampler::new(self.index)?;
                        return Ok(false);
                    }
                    Err(e) => return Err(err(e)),
                }
                let result = (|| -> std::io::Result<bool> {
                    if info.LastPresentTime == 0 && self.grid.is_some() {
                        return Ok(false);
                    }
                    let tex: ID3D11Texture2D = res.as_ref().ok_or_else(|| std::io::Error::other("no frame"))?.cast().map_err(err)?;
                    self.read(&tex)?;
                    Ok(true)
                })();
                let _ = self.dup.ReleaseFrame();
                result
            }
        }

        fn grid(&self) -> Option<&Grid> {
            self.grid.as_ref()
        }

        fn monitor(&self) -> MonitorInfo {
            self.info.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrinks_frames_to_cells() {
        // Left half red, right half blue (BGRA).
        let (w, h) = (200u32, 100u32);
        let mut data = vec![0u8; (w * h * 4) as usize];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let p = &mut data[(y * w as usize + x) * 4..][..4];
                p.copy_from_slice(if x < 100 { &[0, 0, 255, 255] } else { &[255, 0, 0, 255] });
            }
        }
        let f = Frame { width: w, height: h, stride: w as usize * 4, data, cursor: None };
        let g = shrink(&f, 8);
        assert_eq!((g.w, g.h), (8, 4));
        assert_eq!(g.at(0, 0), [255, 0, 0]);
        assert_eq!(g.at(7, 3), [0, 0, 255]);
        assert_eq!(shrink(&f, 1000).w, 200, "never wider than the frame");

        let mut s = FrameSampler::new(Box::new(crate::capture::stream::SyntheticSource::new(320, 180)));
        assert!(s.grid().is_none());
        assert!(s.next(Duration::from_millis(100)).unwrap());
        let g = s.grid().unwrap();
        assert_eq!((g.w, g.h), (64, 36));
    }
}
