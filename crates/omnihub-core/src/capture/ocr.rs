//! Text in images, read with the OCR engine built into Windows
//! (Windows.Media.Ocr, in the languages installed for the user). Nothing is
//! uploaded.

use std::path::Path;

/// Recognise the text in an image file; lines are separated by newlines.
pub fn recognize(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read the image: {e}"))?;
    #[cfg(windows)]
    {
        // WinRT calls block here; keep them off any UI or async thread.
        std::thread::spawn(move || win::recognize(&bytes)).join().map_err(|_| "text recognition crashed".to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = bytes;
        Err("Copying text from screenshots uses the OCR engine built into Windows.".into())
    }
}

#[cfg(windows)]
mod win {
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat, BitmapTransform, ColorManagementMode, ExifOrientationMode, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    fn err(e: windows::core::Error) -> String {
        e.message().to_string()
    }

    pub fn recognize(bytes: &[u8]) -> Result<String, String> {
        // SAFETY: initialises WinRT for this (new) thread.
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| "Windows has no text-recognition language installed (Settings → Time & language → Language).".to_string())?;
        let stream = InMemoryRandomAccessStream::new().map_err(err)?;
        let writer = DataWriter::CreateDataWriter(&stream).map_err(err)?;
        writer.WriteBytes(bytes).map_err(err)?;
        writer.StoreAsync().map_err(err)?.join().map_err(err)?;
        writer.FlushAsync().map_err(err)?.join().map_err(err)?;
        writer.DetachStream().map_err(err)?;
        stream.Seek(0).map_err(err)?;
        let decoder = BitmapDecoder::CreateAsync(&stream).map_err(err)?.join().map_err(err)?;
        let (w, h) = (decoder.PixelWidth().map_err(err)?, decoder.PixelHeight().map_err(err)?);
        // The engine refuses images larger than MaxImageDimension; scale those down.
        let max = OcrEngine::MaxImageDimension().map_err(err)?;
        let transform = BitmapTransform::new().map_err(err)?;
        if w > max || h > max {
            let k = max as f64 / w.max(h) as f64;
            transform.SetScaledWidth(((w as f64 * k) as u32).max(1)).map_err(err)?;
            transform.SetScaledHeight(((h as f64 * k) as u32).max(1)).map_err(err)?;
        }
        let bitmap: SoftwareBitmap = decoder
            .GetSoftwareBitmapTransformedAsync(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Premultiplied, &transform, ExifOrientationMode::RespectExifOrientation, ColorManagementMode::DoNotColorManage)
            .map_err(err)?
            .join()
            .map_err(err)?;
        let result = engine.RecognizeAsync(&bitmap).map_err(err)?.join().map_err(err)?;
        let mut out = Vec::new();
        for line in result.Lines().map_err(err)? {
            out.push(line.Text().map_err(err)?.to_string());
        }
        Ok(out.join("\n"))
    }
}
