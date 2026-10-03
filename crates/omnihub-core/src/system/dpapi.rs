//! Windows Data Protection API: encrypt bytes so only the same Windows user
//! (on the same PC, unless the profile roams) can decrypt them.

#[cfg(windows)]
pub fn protect(data: &[u8], entropy: &[u8]) -> std::io::Result<Vec<u8>> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};
    unsafe {
        let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let ent = CRYPT_INTEGER_BLOB { cbData: entropy.len() as u32, pbData: entropy.as_ptr() as *mut u8 };
        let mut out = CRYPT_INTEGER_BLOB::default();
        CryptProtectData(&input, windows::core::w!("OmniHub"), Some(&ent), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out)
            .map_err(|e| std::io::Error::other(e.message()))?;
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _)));
        Ok(v)
    }
}

#[cfg(windows)]
pub fn unprotect(data: &[u8], entropy: &[u8]) -> std::io::Result<zeroize::Zeroizing<Vec<u8>>> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};
    use zeroize::Zeroize;
    unsafe {
        let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let ent = CRYPT_INTEGER_BLOB { cbData: entropy.len() as u32, pbData: entropy.as_ptr() as *mut u8 };
        let mut out = CRYPT_INTEGER_BLOB::default();
        CryptUnprotectData(&input, None, Some(&ent), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out)
            .map_err(|e| std::io::Error::other(e.message()))?;
        let slice = std::slice::from_raw_parts_mut(out.pbData, out.cbData as usize);
        let v = zeroize::Zeroizing::new(slice.to_vec());
        slice.zeroize();
        let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _)));
        Ok(v)
    }
}

pub fn available() -> bool {
    cfg!(windows)
}

#[cfg(not(windows))]
pub fn protect(_data: &[u8], _entropy: &[u8]) -> std::io::Result<Vec<u8>> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "DPAPI is Windows-only"))
}

#[cfg(not(windows))]
pub fn unprotect(_data: &[u8], _entropy: &[u8]) -> std::io::Result<zeroize::Zeroizing<Vec<u8>>> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "DPAPI is Windows-only"))
}
