use ring::digest::{Context, Digest, SHA256};
use url::Url;

pub struct UrlHash(pub String);

fn sha256_digest(s: &str) -> Digest {
    let bytes = s.bytes().collect::<Vec<u8>>();
    let mut context = Context::new(&SHA256);
    context.update(&bytes);
    context.finish()
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::new();
    for byte in bytes {
        output = format!["{output}{:02X}", byte];
    }
    output
}

pub fn hash_url(url: Url) -> UrlHash {
    UrlHash(encode_hex(sha256_digest(&url.to_string()).as_ref()))
}
