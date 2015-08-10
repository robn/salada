use std::io::{Read, Write};

use hyper::server::{Request, Response};
use hyper::method::Method;
use hyper::method::Method::{Post, Get, Head};
use hyper::status::StatusCode;
use hyper::uri::RequestUri::AbsolutePath;
use hyper::header;

use jmap_handler::handler as jmap_handler;
use static_handler::handler as static_handler;

pub struct StatusBody {
    pub code: StatusCode,
    pub body: Option<Vec<u8>>
}
impl StatusBody {
    pub fn new(code: StatusCode, body: Option<Vec<u8>>) -> StatusBody {
        StatusBody { code: code, body: body }
    }
}

fn finish_response(method: Method, path: &String, mut res: Response, out: StatusBody) {
    *res.status_mut() = out.code;

    info!("{} {} => {}", method, path, out.code);

    match res.start()
        .and_then(|mut res|
                  match out.body {
                      Some(ref b) => {
                          try!(res.write_all(b));
                          Ok(res)
                      }
                      None => {
                          Ok(res)
                      }
                  })
        .and_then(|res| res.end()) {
            Err(e) => error!("response error: {}", e),
            _      => (),
        };
}

pub fn handler(mut req: Request, mut res: Response) {
    res.headers_mut().set(header::Server("salada/0.0.5".to_string()));

    let method = req.method.clone();
    let uri = req.uri.clone();

    match (method, uri) {
        (Post, AbsolutePath(ref path)) if path == "/jmap/" => {
            let sb = jmap_handler(req);
            finish_response(Post, path, res, sb)
        },

        (Get, AbsolutePath(ref path)) => {
            let sb = static_handler(path, &mut res, true);
            finish_response(Get, path, res, sb)
        },

        (Head, AbsolutePath(ref path)) => {
            let sb = static_handler(path, &mut res, false);
            finish_response(Head, path, res, sb)
        },

        (_, ref ap) => {
            let s = match ap {
                &AbsolutePath(ref u) => u,
                _ => panic!("RequestUri not AbsolutePath?"),
            };
            let mut drain: Vec<u8> = vec!();
            req.read_to_end(&mut drain).ok();
            finish_response(req.method, &s, res, StatusBody::new(StatusCode::MethodNotAllowed, None))
        },
    };
}
