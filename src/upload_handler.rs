use std::fs::File;
use std::fs;
use std::io;

use hyper::server::Request;
use hyper::status::StatusCode;
use hyper::header;

use uuid::Uuid;

use http_handler::StatusBody;

pub fn handler(mut req: Request) -> StatusBody {
    let expected = match req.headers.get::<header::ContentLength>() {
        Some(n) => n.0,
        _ => 0,
    };
    if expected == 0 {
        return StatusBody::new(StatusCode::BadRequest, None);
    }

    // XXX assuming not exists
    // XXX sha1 dedup
    let uuid = Uuid::new_v4().to_hyphenated_string();
    let fullpath = String::from("upload/") + uuid.as_ref();

    match File::create(fullpath.clone()) {
        Err(ref e) => match e.kind() {
            // XXX others
            _ => StatusBody::new(StatusCode::InternalServerError, Some(format!("{}", e).into_bytes())),
        },
        Ok(ref mut f) => {
            match io::copy(&mut req, f) {
                Err(e) => StatusBody::new(StatusCode::InternalServerError, Some(format!("{}", e).into_bytes())),
                Ok(size) => {
                    if size != expected {
                        info!("client advertised {} bytes, uploaded {}, discarding", expected, size);
                        fs::remove_file(fullpath).ok();
                        return StatusBody::new(StatusCode::BadRequest, None);
                    }
                    // XXX save to db
                    // XXX response object
                    info!("created upload {} size {}", uuid, size);
                    StatusBody::new(StatusCode::Ok, None)
                },
            }
        },
    }
}
