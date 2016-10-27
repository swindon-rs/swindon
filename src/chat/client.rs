use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use curl;
use curl::easy::{Easy, List};
use tokio_curl::Session;
use tokio_core::reactor::Handle;
use rustc_serialize::json::{self, Json};    // XXX: use serde
use futures::{Future, BoxFuture};
use futures::{finished, failed};
use netbuf::Buf;

// HttpClient (service)
//
//  new(curl_session)
//
//  call(req: HttpRequest) -> Future<Response>

struct HttpJsonClient {
    session: Session,
}

impl HttpJsonClient {

    pub fn new(handle: Handle) -> HttpJsonClient {
        HttpJsonClient {
            session: Session::new(handle),
        }
    }

    pub fn perform(&self, req: JsonRequest, auth: Option<String>)
        -> BoxFuture<JsonResponse, io::Error>
    {
        let body_buf = Arc::new(Mutex::new(Buf::new()));

        let curl = match self.build(req, auth, body_buf.clone()) {
            Ok(curl) => curl,
            Err(e) => {
                return failed(io::Error::new(
                    io::ErrorKind::Other, "Curl error"))
                    .boxed()
            }
        };

        self.session.perform(curl)
            .map_err(|e| e.into_error())
            .and_then(move |resp| {
                // TODO: collect & parse response body
                let buf = body_buf.lock().unwrap();
                finished(JsonResponse::from(resp, &buf))
            }).boxed()
    }

    fn build(&self, req: JsonRequest, auth: Option<String>,
        body_buf: Arc<Mutex<Buf>>)
        -> Result<Easy, curl::Error>
    {
        let mut curl = Easy::new();
        let mut headers = List::new();
        try!(curl.forbid_reuse(true));
        try!(curl.url(
            format!("http://{}{}", req.backend, req.method).as_str()));
        match req.data {
            Some(ref payload) => {
                try!(curl.post(true));
                try!(headers.append("Content-Type: application/json"));
                try!(curl.post_fields_copy(
                    json::encode(payload).unwrap().as_bytes()));
            }
            None => {
                try!(curl.get(true));
            }
        }
        if let Some(auth) = auth {
            try!(headers.append(format!("Authorization: {}", auth).as_str()));
        }
        try!(curl.http_headers(headers));

        try!(curl.write_function(move |buf| {
            body_buf.lock().unwrap()
                .write(buf)
                .map_err(|e| {
                    panic!("write respone body error: {:?}", e);
                })
        }));
        Ok(curl)
    }
}

struct JsonRequest {
    /// Target host's IP address
    backend: String,
    /// Request path not HTTP Method; HTTP method is setup automatically
    method: String,
    
    /// Request data; if data is None -> do GET, POST otherwise;
    data: Option<Json>,
}

struct JsonResponse {
}

impl JsonResponse {
    pub fn from(resp: Easy, body: &Buf) -> JsonResponse {
        JsonResponse {}
    }
}
