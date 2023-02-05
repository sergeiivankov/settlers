use bytes::Bytes;
use http_body_util::Full;
use hyper::{ body::Incoming, Request, Response };

pub async fn api(_path: &str, _req: Request<Incoming>) -> Result<Response<Full<Bytes>>, String> {
  Ok(Response::new(Full::new(Bytes::from("{\"status\":\"ok\"}"))))
}