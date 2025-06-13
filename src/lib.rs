use axum::body::Body;
use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};

const HASH_LEN: usize = 8;

pub fn path<Asset: rust_embed::RustEmbed>(prefix: &str, file_path: &str) -> Option<String> {
    Asset::get(file_path).map(|f| {
        let mut p = String::from(prefix);
        if !prefix.ends_with('/') {
            p.push('/');
        }
        BASE64_URL_SAFE_NO_PAD.encode_string(&f.metadata.sha256_hash()[..HASH_LEN], &mut p);
        p.push('/');
        p.push_str(file_path);
        p
    })
}

pub async fn handle<Asset: rust_embed::RustEmbed>(
    Path(path): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (hash_b64, file_path) = path
        .split_once('/')
        .ok_or((StatusCode::BAD_REQUEST, "invalid asset url"))?;
    let file = Asset::get(file_path).ok_or((StatusCode::NOT_FOUND, "asset not found"))?;
    let hash = BASE64_URL_SAFE_NO_PAD
        .decode(hash_b64)
        .map_err(|_| (StatusCode::BAD_REQUEST, "hash invalid format"))?;
    if hash.len() != HASH_LEN {
        return Err((StatusCode::BAD_REQUEST, "hash invalid length"));
    }
    if !file.metadata.sha256_hash().starts_with(&hash) {
        return Err((StatusCode::BAD_REQUEST, "hash mismatch"));
    }
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, file.metadata.mimetype())
        .header(header::CACHE_CONTROL, "public,max-age=31536000,immutable")
        .body(Body::from(file.data))
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to build response",
            )
        }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::Router;
    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "src/"]
    struct Asset;

    #[test]
    fn test_is_valid_handler() {
        let _ = Router::<()>::new().route("/", get(handle::<Asset>));
    }
}
