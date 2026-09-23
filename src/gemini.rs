use {
  germ::request::{CertificateStore, RequestOptions, Response},
  std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
  },
  url::Url,
};

const MAXIMUM_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const MAXIMUM_CACHED_CERTIFICATES: usize = 4096;

type CertificateKey = (String, u16);

static CERTIFICATES: LazyLock<Mutex<HashMap<CertificateKey, Vec<u8>>>> =
  LazyLock::new(|| Mutex::new(HashMap::new()));
static REQUEST_OPTIONS: LazyLock<RequestOptions> =
  LazyLock::new(|| RequestOptions {
    max_response_bytes: MAXIMUM_RESPONSE_BYTES,
    ..RequestOptions::default()
  });

#[derive(Default)]
struct SessionCertificates {
  last_loaded: Option<(CertificateKey, Option<Vec<u8>>)>,
}

impl CertificateStore for SessionCertificates {
  fn load(
    &mut self,
    hostname: &str,
    port: u16,
  ) -> anyhow::Result<Option<Vec<u8>>> {
    let key = (hostname.to_string(), port);
    let certificate = CERTIFICATES
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
      .get(&key)
      .cloned();

    self.last_loaded = Some((key, certificate.clone()));

    Ok(certificate)
  }

  fn save(
    &mut self,
    hostname: &str,
    port: u16,
    certificate: &[u8],
  ) -> anyhow::Result<()> {
    let key = (hostname.to_string(), port);
    let Some((loaded_key, previous)) = &self.last_loaded else {
      anyhow::bail!("Gemini certificate was not loaded before saving");
    };

    anyhow::ensure!(loaded_key == &key, "Gemini certificate host changed");

    let mut certificates =
      CERTIFICATES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let current = certificates.get(&key);

    anyhow::ensure!(
      current == previous.as_ref()
        || current.map(Vec::as_slice) == Some(certificate),
      "Gemini certificate changed while checking {hostname}:{port}"
    );
    anyhow::ensure!(
      current.is_some() || certificates.len() < MAXIMUM_CACHED_CERTIFICATES,
      "Gemini certificate cache is full"
    );

    certificates.insert(key, certificate.to_vec());
    drop(certificates);

    Ok(())
  }
}

pub async fn request(url: &Url) -> anyhow::Result<Response> {
  anyhow::ensure!(url.scheme() == "gemini", "URL must use the gemini scheme");

  let mut certificates = SessionCertificates::default();

  germ::request::non_blocking::request_with_tofu(
    url,
    &mut certificates,
    &REQUEST_OPTIONS,
  )
  .await
}

#[cfg(test)]
mod tests {
  use {
    super::{CertificateStore, SessionCertificates},
    url::Url,
  };

  #[tokio::test]
  async fn rejects_non_gemini_urls() {
    let url = Url::parse("https://example.org/").unwrap();

    assert!(super::request(&url).await.is_err());
  }

  #[test]
  fn rejects_competing_first_certificates() {
    let hostname = "certificate-test.invalid";
    let mut first = SessionCertificates::default();
    let mut second = SessionCertificates::default();

    assert_eq!(first.load(hostname, 1965).unwrap(), None);
    assert_eq!(second.load(hostname, 1965).unwrap(), None);
    first.save(hostname, 1965, b"first").unwrap();
    assert!(second.save(hostname, 1965, b"second").is_err());
    assert_eq!(second.load(hostname, 1965).unwrap(), Some(b"first".to_vec()));
  }
}
