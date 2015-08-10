use std::io::{Read, ErrorKind};
use std::fs::File;
use std::path::Path;

use hyper::server::Response;
use hyper::status::StatusCode;
use hyper::header;

use mime_guess;

use http_handler::StatusBody;

pub fn handler(path: &String, res: &mut Response, include_body: bool) -> StatusBody {
    let mut pathonly: String = (*path).clone();
    if let Some(n) = pathonly.find(|c| c == '?' || c == '#') {
        pathonly.truncate(n);
    }

    let fullpath = String::from("static") + if pathonly == "/" { "/index.html" } else { &pathonly };
    let mimetype = mime_guess::guess_mime_type(&Path::new(&fullpath));

    let sb = match File::open(fullpath) {
        Err(ref e) => match e.kind() {
            ErrorKind::NotFound => StatusBody::new(StatusCode::NotFound, None),
            // XXX others
            _ => StatusBody::new(StatusCode::InternalServerError, Some(format!("{}", e).into_bytes())),
        },
        Ok(ref mut f) => {
            match include_body {
                false => StatusBody::new(StatusCode::Ok, None),
                true  => {
                    let mut buf: Vec<u8> = vec!();
                    match f.read_to_end(&mut buf) {
                        Err(ref e) =>
                            StatusBody::new(StatusCode::InternalServerError, Some(format!("{}", e).into_bytes())),
                        _ =>
                            StatusBody::new(StatusCode::Ok, Some(buf)),
                    }
                },
            }
        },
    };

    let headers = res.headers_mut();
    headers.set(header::ContentType(mimetype));
    if let Some(ref b) = sb.body {
        headers.set(header::ContentLength((*b).len() as u64));
    }

    sb
}


