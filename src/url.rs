use url::Url;

pub fn from_path(
  path: &str,
  fallback: bool,
  is_proxy: &mut bool,
  is_raw: &mut bool,
  is_nocss: &mut bool,
) -> Result<Url, url::ParseError> {
  Ok(
    #[allow(clippy::blocks_in_conditions)]
    match Url::try_from(&*if path.starts_with("/proxy") {
      *is_proxy = true;

      format!(
        "gemini://{}{}",
        path.replace("/proxy/", ""),
        if fallback { "/" } else { "" }
      )
    } else if path.starts_with("/x") {
      *is_proxy = true;

      format!(
        "gemini://{}{}",
        path.replace("/x/", ""),
        if fallback { "/" } else { "" }
      )
    } else if path.starts_with("/raw") {
      *is_proxy = true;
      *is_raw = true;

      format!(
        "gemini://{}{}",
        path.replace("/raw/", ""),
        if fallback { "/" } else { "" }
      )
    } else if path.starts_with("/nocss") {
      *is_proxy = true;
      *is_nocss = true;

      format!(
        "gemini://{}{}",
        path.replace("/nocss/", ""),
        if fallback { "/" } else { "" }
      )
    } else {
      format!(
        "{}{}{}",
        {
          std::env::var("ROOT").unwrap_or_else(|_| {
            warn!(
              "could not use ROOT from environment variables, proceeding with \
               default root: gemini://fuwn.me"
            );

            "gemini://fuwn.me".to_string()
          })
        },
        path,
        if fallback { "/" } else { "" }
      )
    }) {
      Ok(url) => url,
      Err(e) => return Err(e),
    },
  )
}
