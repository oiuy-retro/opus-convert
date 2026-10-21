//! HTTPS GET for Last.fm.

use windows::core::{w, PCWSTR};
use windows::Win32::Networking::WinHttp::{
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetTimeouts,
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};

struct Parts {
    https: bool,
    host: String,
    port: u16,
    path: String,
}

pub fn get(url: &str) -> Result<Vec<u8>, String> {
    let parts = split_url(url).ok_or_else(|| format!("Could not read {url}"))?;
    unsafe { fetch(&parts) }
}

pub fn get_text(url: &str) -> Result<String, String> {
    let bytes = get(url)?;
    String::from_utf8(bytes).map_err(|err| err.to_string())
}

fn split_url(url: &str) -> Option<Parts> {
    let (https, rest) = if let Some(rest) = url.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (false, rest)
    } else {
        return None;
    };
    let (hostport, path) = match rest.find('/') {
        Some(index) => (&rest[..index], rest[index..].to_string()),
        None => (rest, "/".into()),
    };
    if hostport.is_empty() {
        return None;
    }
    let (host, port) = if let Some((host, port)) = hostport.rsplit_once(':') {
        if !port.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) {
            (host.to_string(), port.parse().unwrap_or(if https { 443 } else { 80 }))
        } else {
            (hostport.to_string(), if https { 443 } else { 80 })
        }
    } else {
        (hostport.to_string(), if https { 443 } else { 80 })
    };
    Some(Parts { https, host, port, path })
}

unsafe fn fetch(parts: &Parts) -> Result<Vec<u8>, String> {
    let session = WinHttpOpen(w!("Opus Convert"), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, None, None, 0);
    if session.is_null() {
        return Err("WinHTTP did not open.".into());
    }
    let _session = Guard(session);
    WinHttpSetTimeouts(session, 15_000, 15_000, 20_000, 20_000).map_err(|err| err.to_string())?;
    let host = wide(&parts.host);
    let connect = WinHttpConnect(session, PCWSTR(host.as_ptr()), parts.port, 0);
    if connect.is_null() {
        return Err("WinHTTP did not connect.".into());
    }
    let _connect = Guard(connect);
    let path = wide(&parts.path);
    let flags = if parts.https {
        WINHTTP_FLAG_SECURE
    } else {
        WINHTTP_OPEN_REQUEST_FLAGS(0)
    };
    let request = WinHttpOpenRequest(
        connect,
        w!("GET"),
        PCWSTR(path.as_ptr()),
        None,
        None,
        std::ptr::null(),
        flags,
    );
    if request.is_null() {
        return Err("WinHTTP did not open the request.".into());
    }
    let _request = Guard(request);
    WinHttpSendRequest(request, None, None, 0, 0, 0).map_err(|err| err.to_string())?;
    WinHttpReceiveResponse(request, std::ptr::null_mut()).map_err(|err| err.to_string())?;
    let mut status = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    WinHttpQueryHeaders(
        request,
        WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
        None,
        Some(std::ptr::addr_of_mut!(status).cast()),
        &mut size,
        std::ptr::null_mut(),
    )
    .map_err(|err| err.to_string())?;
    if status >= 400 {
        return Err(format!("Last.fm returned HTTP {status}."));
    }
    let mut body = Vec::new();
    loop {
        let mut available = 0u32;
        WinHttpQueryDataAvailable(request, &mut available).map_err(|err| err.to_string())?;
        if available == 0 {
            break;
        }
        let mut chunk = vec![0u8; available as usize];
        let mut read = 0u32;
        WinHttpReadData(request, chunk.as_mut_ptr().cast(), available, &mut read).map_err(|err| err.to_string())?;
        chunk.truncate(read as usize);
        body.extend_from_slice(&chunk);
        if body.len() > 2 * 1024 * 1024 {
            return Err("The Last.fm reply was larger than expected.".into());
        }
    }
    Ok(body)
}

struct Guard(*mut core::ffi::c_void);

impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            let _ = WinHttpCloseHandle(self.0);
        }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_lastfm_url() {
        let parts = split_url("https://ws.audioscrobbler.com/2.0/?method=track.getTopTags").unwrap();
        assert!(parts.https);
        assert_eq!(parts.host, "ws.audioscrobbler.com");
        assert_eq!(parts.port, 443);
        assert!(parts.path.starts_with("/2.0/"));
    }
}
