use axum::{
    body::Body,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
};

#[derive(Debug)]
pub struct BinaryResource {
    data: Vec<u8>,
    content_type: HeaderValue,
    etag: HeaderValue,
}

impl BinaryResource {
    pub fn new(data: Vec<u8>, etag: &str, content_type: &str) -> Self {
        Self {
            data,
            content_type: HeaderValue::from_str(&content_type)
                .expect("Invalid content type header"),
            etag: HeaderValue::from_str(&etag).expect("Invalid etag header"),
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn update_data(&mut self, data: Vec<u8>, etag: String) {
        self.data = data;
        self.etag = HeaderValue::from_str(&etag).expect("Failed to create etag header")
    }

    pub async fn respond(&self, req_headers: &HeaderMap) -> Response {
        if let Some(req_etag) = req_headers.get(header::IF_NONE_MATCH) {
            if req_etag == self.etag {
                return Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .header(header::ETAG, self.etag.clone())
                    .body(Body::empty())
                    .unwrap();
            }
        }

        Response::builder()
            .header(header::CONTENT_TYPE, self.content_type.clone())
            .header(header::ETAG, self.etag.clone())
            .body(Body::from(self.data.clone()))
            .unwrap()
    }
}
