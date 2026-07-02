use url::Url;

pub fn from_path(
  path: &str,
  configuration: &mut crate::response::configuration::Configuration,
) -> Result<Url, url::ParseError> {
  Url::try_from(&*if let Some(remainder) =
    route_remainder(path, "/proxy").or_else(|| route_remainder(path, "/x"))
  {
    configuration.proxy = true;

    format!("gemini://{remainder}")
  } else if let Some(remainder) = route_remainder(path, "/raw") {
    configuration.proxy = true;
    configuration.raw = true;

    format!("gemini://{remainder}")
  } else if let Some(remainder) = route_remainder(path, "/nocss") {
    configuration.proxy = true;
    configuration.no_css = true;

    format!("gemini://{remainder}")
  } else {
    format!("{}{}", &crate::environment::ENVIRONMENT.root, path)
  })
}

// A route prefix only matches a whole path segment: "/raw" and "/raw/..."
// are the raw route, but "/rawhide" is a page on the root capsule.
fn route_remainder<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
  match path.strip_prefix(prefix)? {
    "" => Some(""),
    remainder => remainder.strip_prefix('/'),
  }
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
