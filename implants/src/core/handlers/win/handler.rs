use std::ffi::c_void;
use windows::{
    core::{Error, HRESULT, HSTRING, PCWSTR},
    Win32::{
        Networking::WinHttp::*,
        Security::Cryptography::{
            CertOpenStore,
            CertOpenSystemStoreA,
            CERT_STORE_OPEN_EXISTING_FLAG,
            CERT_STORE_PROV_SYSTEM,
            CERT_SYSTEM_STORE_LOCAL_MACHINE_ID,
        },
    }
};

pub struct HInternet {
    handle: *mut c_void,
}

unsafe impl Send for HInternet {}

impl Drop for HInternet {
    fn drop(&mut self) {
        self.close();
    }
}

impl HInternet {
    pub fn close(&mut self) {
        if self.handle.is_null() {
            return;
        }
        let result = unsafe { WinHttpCloseHandle(self.handle) };
        if result.is_err() {
            let e = Error::from_win32();
            assert!(e.code().is_ok(), "Error: {}", e);
        }
        self.handle = std::ptr::null_mut();
    }
}

pub struct HSession {
    pub h: HInternet,
}

impl HSession {
    pub fn new(user_agent: HSTRING) -> Result<HSession, Error> {
        let hi = open_session(user_agent)?;
        Ok(HSession { h: hi })
    }
}

pub struct HConnect {
    pub h: HInternet,
}

impl HConnect {
    pub fn new(hsession: &HSession, host: HSTRING, port: u16) -> Result<HConnect, Error> {
        let hi = connect(&hsession.h, host, port)?;
        Ok(HConnect { h: hi })
    }
}

pub struct HRequest {
    pub h: HInternet,
}

impl HRequest {
    pub fn new(
        hconnect: &HConnect,
        method: HSTRING,
        url_path: HSTRING,
        accept_types: Option<Vec<HSTRING>>
    ) -> Result<HRequest, Error> {
        let hi = open_request(&hconnect.h, method, url_path, accept_types)?;

        // Set option to ignore invalid certificates for HTTPS
        match set_option_ignore_cert_invalid(&hi) {
            Ok(_) => {}
            Err(e) => {
                println!("Error occured when setting the WinHttpSetOption.");
                return Err(Error::from_win32());
            }
        }

        Ok(HRequest { h: hi })
    }

    pub fn set_status_callback(
        &self,
        lpfninternetcallback: WINHTTP_STATUS_CALLBACK,
        dwnotificationflags: u32,
        dwreserved: usize,
    ) -> WINHTTP_STATUS_CALLBACK {
        unsafe {
            WinHttpSetStatusCallback(
                self.h.handle,
                lpfninternetcallback,
                dwnotificationflags,
                dwreserved,
            )
        }
    }

    pub fn add_headers(&mut self, method: &str) -> Result<(), Error> {
        let content_type = match method {
            "GET" => HSTRING::from("Content-Type: text/plain"),
            "POST" => HSTRING::from("Content-Type: application/json"),
            _ => HSTRING::from("Content-Type: text/plain"),
        };
    
        unsafe {
            WinHttpAddRequestHeaders(
                self.h.handle,
                content_type.as_wide(),
                WINHTTP_ADDREQ_FLAG_ADD,
            );
        }
        Ok(())
    }

    pub fn send_req(
        &mut self,
        headers: HSTRING,
        total_length: u32,
        ctx: usize
    ) -> Result<(), Error> {
        let mut headers_op: Option<&[u16]> = None;
        if !headers.is_empty() {
            headers_op = Some(headers.as_wide());
        }

        let mut b_results = Ok(());
    
        b_results = unsafe {
            WinHttpSendRequest(
                self.h.handle,
                headers_op,
                Some(std::ptr::null()),
                0,
                total_length,
                ctx,
            )
        };
    
        if b_results.is_ok() {
            return Ok(());
        } else {
            return Err(Error::from_win32());
        }
    }

    pub fn write_data(
        &self,
        buf: &[u8],
        dwnumberofbytestowrite: u32,
        lpdwnumberofbyteswritten: Option<&mut u32>,
    ) -> Result<(), Error> {
        let len = buf.len();
        let lpdwnumberofbyteswritten_op: *mut u32 = match lpdwnumberofbyteswritten {
            Some(op) => op,
            None => std::ptr::null_mut(),
        };

        assert!(dwnumberofbytestowrite as usize <= len);

        unsafe {
            WinHttpWriteData(
                self.h.handle,
                Some(buf.as_ptr() as *const c_void),
                dwnumberofbytestowrite,
                lpdwnumberofbyteswritten_op,
            );
        };
        Ok(())
    }

    pub fn recv_resp(&mut self) -> Result<(), Error> {
        unsafe {
            WinHttpReceiveResponse(
                self.h.handle,
                std::ptr::null_mut()
            )
        }
    }

    pub fn query_data_available(&mut self, dw_size: Option<&mut u32>) -> Result<(), Error> {
        let num_of_bytes_available_op: *mut u32 = match dw_size {
            Some(op) => op,
            None => std::ptr::null_mut(),
        };

        unsafe {
            WinHttpQueryDataAvailable(
                self.h.handle,
                num_of_bytes_available_op,
            )
        }
    }

    pub fn read_data(
        &mut self,
        dw_size: u32,
        buffer: &mut [u8],
    ) -> Result<(), Error> {
        let mut lpdw_num_of_bytes_read: u32 = 0;

        let num_of_bytes_read_op: *mut u32 = match Some(&mut lpdw_num_of_bytes_read) {
            Some(op) => op,
            None => std::ptr::null_mut(),
        };

        unsafe {
            WinHttpReadData(
                self.h.handle,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                dw_size,
                num_of_bytes_read_op,
            )
        }
    }
}

fn open_session(user_agent: HSTRING) -> Result<HInternet, Error> {
    let handle = unsafe {
        WinHttpOpen(
            &user_agent,
            WINHTTP_ACCESS_TYPE_NO_PROXY,
            &HSTRING::new(),
            &HSTRING::new(),
            WINHTTP_FLAG_ASYNC)
    };

    if handle.is_null() {
        return Err(Error::from_win32());
    }

    Ok(HInternet { handle })
}

fn connect(h: &HInternet, host: HSTRING, port: u16) -> Result<HInternet, Error> {
    let handle = unsafe {
        WinHttpConnect(
            h.handle,
            &host,
            port,
            0
        )
    };

    if handle.is_null() {
        return Err(Error::from_win32());
    }

    Ok(HInternet { handle })
}

fn open_request(
    h: &HInternet,
    method: HSTRING,
    url_path: HSTRING,
    accept_types: Option<Vec<HSTRING>>
) -> Result<HInternet, Error> {
    let mut at: Vec<PCWSTR> = match accept_types {
        Some(v) => {
            let mut out = v
                .into_iter()
                .map(|s| PCWSTR::from_raw(s.as_ptr()))
                .collect::<Vec<_>>();
            out.push(PCWSTR::from_raw(std::ptr::null()));
            out
        }
        None => Vec::new(),
    };

    let mut temp_ptr: *mut PCWSTR = std::ptr::null_mut();
    if !at.is_empty() {
        temp_ptr = at.as_mut_ptr();
    }
    
    let mut handle: *mut c_void = unsafe {
        WinHttpOpenRequest(
            h.handle,
            &method,
            &url_path,
            PCWSTR::null(),
            PCWSTR::null(),
            temp_ptr,
            // WINHTTP_OPEN_REQUEST_FLAGS(0), // for HTTP
            WINHTTP_FLAG_SECURE, // for HTTPS
        )
    };

    if handle.is_null() {
        return Err(Error::from_win32());
    }

    Ok(HInternet { handle })
}

fn set_option_ignore_cert_invalid(h: &HInternet) -> Result<(), Error> {
    let dwflags: u32 =
        SECURITY_FLAG_IGNORE_UNKNOWN_CA |
        SECURITY_FLAG_IGNORE_CERT_WRONG_USAGE |
        SECURITY_FLAG_IGNORE_CERT_CN_INVALID |
        SECURITY_FLAG_IGNORE_CERT_DATE_INVALID;

    //     let dwflags: u32 =
    //         WINHTTP_FLAG_SECURE_PROTOCOL_SSL3 |
    //         WINHTTP_FLAG_SECURE_PROTOCOL_TLS1 |
    //         WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_1 |
    //         WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2 |
    //         WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_3;

    let success = unsafe {
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Networking/WinHttp/fn.WinHttpSetOption.html
        WinHttpSetOption(
            Some(h.handle),
            WINHTTP_OPTION_SECURITY_FLAGS,
            Some(&dwflags.to_le_bytes()),
        )
    };

    success
}

// HTTPS certificates
// References:
// - https://learn.microsoft.com/ja-jp/windows/win32/winhttp/ssl-in-winhttp
// - https://www.codeproject.com/Articles/24003/One-Click-SSL-Certificate-Registration-using-WinHT
fn set_certificates(h: &HInternet) {
    // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Networking/WinHttp/fn.WinHttpQueryOption.html
    // bret = unsafe {
    //     WinHttpQueryOption(
    //         hrequest,
    //         WINHTTP_OPTION_SERVER_CERT_CONTEXT,
    //         &pcert,
    //         &dwlen,
    //     );
    // };

    // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Security/Cryptography/fn.CertOpenStore.html
    // hcertstore = unsafe {
    //     CertOpenStore(
    //         HSTRING::from("Root CA ECC"),
    //         CERT_STORE_PROV_SYSTEM,
    //         0,
    //         CERT_STORE_OPEN_EXISTING_FLAG | CERT_SYSTEM_STORE_LOCAL_MACHINE_ID,
    //     )
    // };

    // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Security/Cryptography/fn.CertOpenSystemStoreA.html
    // let hcertstore = unsafe {
    //     CertOpenSystemStoreA(
    //         0,
    //         HSTRING::from("Root CA ECC"),
    //     )
    // };

    // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Security/Cryptography/fn.CertAddCertificateContextToStore.html
    // bret = CertAddCertificateContextToStore(
    //     hcertstore,
    //     pcert,
    //     CERT_STORE_ADD_REPLACE_EXISTING,
    //     NULL
    // );

    // - https://gist.github.com/henkman/2e7a4dcf4822bc0029d7d2af731da5c5
    // - https://learn.microsoft.com/en-us/answers/questions/673794/tls-1-3-support-for-winhttp-in-windows11
    //     let dwflags: u32 =
    //         SECURITY_FLAG_IGNORE_UNKNOWN_CA |
    //         SECURITY_FLAG_IGNORE_CERT_WRONG_USAGE |
    //         SECURITY_FLAG_IGNORE_CERT_CN_INVALID |
    //         SECURITY_FLAG_IGNORE_CERT_DATE_INVALID;

    //     let dwflags: u32 =
    //         WINHTTP_FLAG_SECURE_PROTOCOL_SSL3 |
    //         WINHTTP_FLAG_SECURE_PROTOCOL_TLS1 |
    //         WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_1 |
    //         WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2 |
    //         WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_3;

    //     let success = unsafe {
    //         // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Networking/WinHttp/fn.WinHttpSetOption.html
    //         WinHttpSetOption(
    //             Some(h.handle),
    //             WINHTTP_OPTION_SECURE_PROTOCOLS,
    //             Some(&mut dwflags.as_ptr()),
    //         );
    //     };

    // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Security/Cryptography/fn.CertFreeCertificateContext.html
    // CertFreeCertificateContext(pcert);

    // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Security/Cryptography/fn.CertCloseStore.html
    // let bret = CertCloseStore(hcertstore, 0);


    // -----------------------------------------------------------------------------
    // Reference:
    // Reference: https://www.codeproject.com/Articles/24003/One-Click-SSL-Certificate-Registration-using-WinHT

    // get a handle on the certificate
    // bRet = WinHttpQueryOption(
    //     hRequest,
    //     WINHTTP_OPTION_SERVER_CERT_CONTEXT,
    //     &pCert,
    //     &dwLen
    // );

    // open a certificate store
    // hCertStore = CertOpenStore(
    //    CERT_STORE_PROV_SYSTEM,
    //     0,
    //     0,
    //     CERT_STORE_OPEN_EXISTING_FLAG | CERT_SYSTEM_STORE_LOCAL_MACHINE,
    //     L"Root");

    // // add the certificate
    // bRet =  CertAddCertificateContextToStore(
    //     hCertStore,
    //     pCert,
    //     CERT_STORE_ADD_REPLACE_EXISTING,
    //     NULL
    // );

    // // release the certificate
    // CertFreeCertificateContext(pCert);

    // // close the store
    // bRet = CertCloseStore( hCertStore, 0 );
}