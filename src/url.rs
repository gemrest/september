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

pub fn matches_pattern(pattern: &str, path: &str) -> bool {
  if !pattern.contains('*') {
    return path == pattern;
  }

  let parts: Vec<&str> = pattern.split('*').collect();
  let mut position = if pattern.starts_with('*') {
    0
  } else {
    let first = parts.first().unwrap();

    if !path.starts_with(first) {
      return false;
    }

    first.len()
  };
  let before_last = parts.len().saturating_sub(1);

  for part in &parts[1..before_last] {
    if part.is_empty() {
      continue;
    }

    if let Some(found) = path[position..].find(part) {
      position += found + part.len();
    } else {
      return false;
    }
  }

  if !pattern.ends_with('*') {
    let last = parts.last().unwrap();

    if !path[position..].ends_with(last) {
      return false;
    }
  }

  true
}
