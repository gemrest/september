use url::Url;

pub fn from_path(
  path: &str,
  fallback: bool,
  configuration: &mut crate::response::configuration::Configuration,
) -> Result<Url, url::ParseError> {
  Url::try_from(&*if path.starts_with("/proxy") {
    configuration.set_proxy(true);

    format!(
      "gemini://{}{}",
      path.replace("/proxy/", ""),
      if fallback { "/" } else { "" }
    )
  } else if path.starts_with("/x") {
    configuration.set_proxy(true);

    format!(
      "gemini://{}{}",
      path.replace("/x/", ""),
      if fallback { "/" } else { "" }
    )
  } else if path.starts_with("/raw") {
    configuration.set_proxy(true);
    configuration.set_raw(true);

    format!(
      "gemini://{}{}",
      path.replace("/raw/", ""),
      if fallback { "/" } else { "" }
    )
  } else if path.starts_with("/nocss") {
    configuration.set_proxy(true);
    configuration.set_no_css(true);

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
  })
}
