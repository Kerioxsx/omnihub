//! Windows Hello unlock.
//!
//! A Windows Hello key pair (kept by the TPM where available) signs a fixed
//! random challenge. RSA PKCS#1 v1.5 signatures are deterministic, so the
//! SHA-256 of the signature is a stable secret that only exists after the
//! user passes the Hello prompt (face, fingerprint or PIN). That secret
//! wraps the vault key in `vault.hello`. Unlike a plain DPAPI copy of the
//! key, other programs running as the same user cannot unwrap it without
//! triggering a visible Hello prompt.

#[cfg(windows)]
mod imp {
    use sha2::{Digest, Sha256};
    use windows::core::HSTRING;
    use windows::Security::Credentials::{KeyCredentialCreationOption, KeyCredentialManager, KeyCredentialStatus};
    use windows::Security::Cryptography::CryptographicBuffer;
    use zeroize::Zeroizing;

    const NAME: &str = "OmniHub Vault";

    fn err(e: windows::core::Error) -> String {
        e.message().to_string()
    }

    pub fn supported() -> bool {
        KeyCredentialManager::IsSupportedAsync().and_then(|op| op.join()).unwrap_or(false)
    }

    /// Bring the Hello prompt to the front: from a desktop app it often opens
    /// behind the main window.
    fn nudge_prompt() {
        std::thread::spawn(|| {
            use windows::core::w;
            use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow};
            for _ in 0..30 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Ok(hwnd) = unsafe { FindWindowW(w!("Credential Dialog Xaml Host"), None) } {
                    if !hwnd.is_invalid() {
                        let _ = unsafe { SetForegroundWindow(hwnd) };
                        return;
                    }
                }
            }
        });
    }

    fn sign(challenge: &[u8], create: bool) -> Result<Zeroizing<[u8; 32]>, String> {
        let name = HSTRING::from(NAME);
        nudge_prompt();
        let result = if create {
            KeyCredentialManager::RequestCreateAsync(&name, KeyCredentialCreationOption::ReplaceExisting)
        } else {
            KeyCredentialManager::OpenAsync(&name)
        }
        .and_then(|op| op.join())
        .map_err(err)?;
        let status = result.Status().map_err(err)?;
        if status != KeyCredentialStatus::Success {
            return Err(match status {
                KeyCredentialStatus::UserCanceled => "Windows Hello was cancelled".into(),
                KeyCredentialStatus::NotFound => "Windows Hello is not set up for OmniHub".into(),
                _ => format!("Windows Hello failed ({})", status.0),
            });
        }
        let cred = result.Credential().map_err(err)?;
        let buf = CryptographicBuffer::CreateFromByteArray(challenge).map_err(err)?;
        nudge_prompt();
        let signed = cred.RequestSignAsync(&buf).and_then(|op| op.join()).map_err(err)?;
        if signed.Status().map_err(err)? != KeyCredentialStatus::Success {
            return Err("Windows Hello did not confirm".into());
        }
        let sig_buf = signed.Result().map_err(err)?;
        let mut sig = windows::core::Array::<u8>::new();
        CryptographicBuffer::CopyToByteArray(&sig_buf, &mut sig).map_err(err)?;
        let digest = Sha256::digest(&sig[..]);
        let mut out = Zeroizing::new([0u8; 32]);
        out.copy_from_slice(&digest);
        Ok(out)
    }

    pub fn create_secret(challenge: &[u8]) -> Result<Zeroizing<[u8; 32]>, String> {
        let first = sign(challenge, true)?;
        // Make sure signing really is deterministic on this machine before
        // relying on it (it is for the RSA keys Windows Hello creates).
        let second = sign(challenge, false)?;
        if first.as_ref() != second.as_ref() {
            return Err("this Windows Hello key does not produce stable signatures".into());
        }
        Ok(first)
    }

    pub fn secret(challenge: &[u8]) -> Result<Zeroizing<[u8; 32]>, String> {
        sign(challenge, false)
    }

    pub fn delete() {
        let _ = KeyCredentialManager::DeleteAsync(&HSTRING::from(NAME)).and_then(|op| op.join());
    }
}

#[cfg(not(windows))]
mod imp {
    use zeroize::Zeroizing;

    pub fn supported() -> bool {
        false
    }
    pub fn create_secret(_c: &[u8]) -> Result<Zeroizing<[u8; 32]>, String> {
        Err("Windows Hello is only available on Windows".into())
    }
    pub fn secret(_c: &[u8]) -> Result<Zeroizing<[u8; 32]>, String> {
        Err("Windows Hello is only available on Windows".into())
    }
    pub fn delete() {}
}

pub use imp::{create_secret, delete, secret, supported};
