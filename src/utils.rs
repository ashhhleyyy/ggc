use std::fs;
use std::io::BufReader;
use std::path::Path;

use tokio_rustls::rustls;
use tokio_rustls::rustls::internal::pemfile::{certs, rsa_private_keys};

pub const SERVER_BUILD_INFO: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    "-git.",
    env!("GIT_HASH")
);

pub fn load_certs(path: &Path) -> Vec<rustls::Certificate> {
    let certfile = fs::File::open(path).expect("cannot open certificate file");
    let mut reader = BufReader::new(certfile);
    certs(&mut reader)
        .expect("failed to read certificates file")
}

pub fn load_private_key(path: &Path) -> rustls::PrivateKey {
    let keyfile = fs::File::open(path).expect("cannot open private key file");
    let mut reader = BufReader::new(keyfile);
    rsa_private_keys(&mut reader).expect("failed to read private key file")
        .first().expect("no private keys in file").clone()
}
