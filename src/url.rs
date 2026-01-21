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
      &crate::environment::ENVIRONMENT.root,
      path,
      if fallback { "/" } else { "" }
    )
  })
}

pub fn matches_pattern(pattern: &str, path: &str) -> bool {
  if !pattern.contains('*') {
    return path == pattern;
  }

  let mut parts = pattern.split('*').peekable();
  let mut position = if pattern.starts_with('*') {
    0
  } else {
    let first = parts.next().unwrap_or("");

    if !path.starts_with(first) {
      return false;
    }

    first.len()
  };

  while let Some(part) = parts.next() {
    let is_last = parts.peek().is_none();

    if is_last {
      if !pattern.ends_with('*') && !path[position..].ends_with(part) {
        return false;
      }
    } else if !part.is_empty() {
      if let Some(found) = path[position..].find(part) {
        position += found + part.len();
      } else {
        return false;
      }
    }
  }

  true
}
