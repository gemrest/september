use {
  germ::request::Status,
  std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
  },
  url::Url,
};

const CACHE_LIFETIME: Duration = Duration::from_secs(60 * 60);
const MAXIMUM_POLICY_LENGTH: usize = 512 * 1024;
const REDIRECT_LIMIT: usize = 5;
const VIRTUAL_USER_AGENT: &str = "webproxy";

static POLICIES: LazyLock<Mutex<HashMap<String, CachedPolicy>>> =
  LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
struct CachedPolicy {
  disallowed_paths: Vec<String>,
  fetched_at:       Instant,
}

pub enum Access {
  Allowed,
  Denied,
  Unavailable,
}

pub async fn check_access(url: &Url) -> Access {
  if url.path() == "/robots.txt" {
    return Access::Allowed;
  }

  let origin = origin(url);

  if let Some(policy) = cached_policy(&origin) {
    return if policy.allows(url.path()) {
      Access::Allowed
    } else {
      Access::Denied
    };
  }

  let Some(policy) = fetch_policy(url).await else {
    return Access::Unavailable;
  };

  let mut policies =
    POLICIES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

  policies.retain(|_, policy| policy.fetched_at.elapsed() < CACHE_LIFETIME);
  policies.insert(origin, CachedPolicy {
    disallowed_paths: policy.clone(),
    fetched_at:       Instant::now(),
  });

  drop(policies);

  if policy.iter().any(|prefix| url.path().starts_with(prefix)) {
    Access::Denied
  } else {
    Access::Allowed
  }
}

fn cached_policy(origin: &str) -> Option<CachedPolicy> {
  let policy = POLICIES
    .lock()
    .unwrap_or_else(std::sync::PoisonError::into_inner)
    .get(origin)
    .cloned()?;

  (policy.fetched_at.elapsed() < CACHE_LIFETIME).then_some(policy)
}

fn origin(url: &Url) -> String { url[..url::Position::BeforePath].to_string() }

async fn fetch_policy(url: &Url) -> Option<Vec<String>> {
  let mut robots_url = url.clone();

  robots_url.set_path("/robots.txt");
  robots_url.set_query(None);
  robots_url.set_fragment(None);

  for _ in 0..=REDIRECT_LIMIT {
    let response = germ::request::request(&robots_url).await.ok()?;

    if matches!(
      response.status(),
      Status::PermanentRedirect | Status::TemporaryRedirect
    ) {
      robots_url = robots_url.join(&response.meta()).ok()?;

      continue;
    }

    let content = response.content();

    return policy_from_response(
      *response.status(),
      &response.meta(),
      content.as_deref(),
    );
  }

  None
}

fn policy_from_response(
  status: Status,
  meta: &str,
  content: Option<&str>,
) -> Option<Vec<String>> {
  if status == Status::NotFound || status == Status::Gone {
    return Some(Vec::new());
  }

  if status != Status::Success
    || !meta
      .split(';')
      .next()
      .unwrap_or("")
      .trim()
      .eq_ignore_ascii_case("text/plain")
  {
    return None;
  }

  let content = content.unwrap_or("");

  (content.len() <= MAXIMUM_POLICY_LENGTH)
    .then(|| disallowed_paths(content, VIRTUAL_USER_AGENT))
}

fn disallowed_paths(policy: &str, user_agent: &str) -> Vec<String> {
  let mut groups = Vec::new();
  let mut agents = Vec::new();
  let mut paths = Vec::new();
  let mut has_directive = false;

  for line in policy.lines() {
    let line = line.split('#').next().unwrap_or("").trim();
    let Some((field, value)) = line.split_once(':') else {
      continue;
    };
    let field = field.trim();
    let value = value.trim();

    if field.eq_ignore_ascii_case("user-agent") {
      if has_directive {
        groups.push((agents, paths));
        agents = Vec::new();
        paths = Vec::new();
        has_directive = false;
      }

      agents.push(value.to_ascii_lowercase());
    } else if field.eq_ignore_ascii_case("disallow") && !agents.is_empty() {
      has_directive = true;

      if !value.is_empty() {
        paths.push(value.to_string());
      }
    }
  }

  if !agents.is_empty() {
    groups.push((agents, paths));
  }

  groups
    .into_iter()
    .filter(|(agents, _)| {
      agents.iter().any(|agent| agent == "*" || agent == user_agent)
    })
    .flat_map(|(_, paths)| paths)
    .collect()
}

impl CachedPolicy {
  fn allows(&self, path: &str) -> bool {
    !self.disallowed_paths.iter().any(|prefix| path.starts_with(prefix))
  }
}

#[cfg(test)]
mod tests {
  use {
    super::{disallowed_paths, origin, policy_from_response},
    germ::request::Status,
    url::Url,
  };

  #[test]
  fn distinguishes_missing_policy_from_unavailable_policy() {
    assert_eq!(policy_from_response(Status::NotFound, "", None), Some(vec![]));
    assert_eq!(policy_from_response(Status::Gone, "", None), Some(vec![]));
    assert_eq!(policy_from_response(Status::TemporaryFailure, "", None), None);
    assert_eq!(
      policy_from_response(Status::Success, "text/gemini", None),
      None
    );
    assert_eq!(
      policy_from_response(Status::Success, "text/plain", None),
      Some(vec![])
    );
    assert_eq!(
      policy_from_response(
        Status::Success,
        "text/plain",
        Some(&"x".repeat(super::MAXIMUM_POLICY_LENGTH + 1))
      ),
      None
    );
    assert_eq!(
      policy_from_response(
        Status::Success,
        "TEXT/PLAIN; charset=utf-8",
        Some("User-agent: webproxy\nDisallow: /private\n")
      ),
      Some(vec!["/private".to_string()])
    );
  }

  #[test]
  fn separates_capsule_origins() {
    let first = Url::parse("gemini://first.example/page").unwrap();
    let second = Url::parse("gemini://second.example/page").unwrap();

    assert_eq!(origin(&first), "gemini://first.example");
    assert_ne!(origin(&first), origin(&second));
  }

  #[test]
  fn combines_webproxy_and_wildcard_groups() {
    let policy = "\
User-agent: indexer
Disallow: /search-only

User-agent: webproxy
Disallow: /private

User-agent: *
Disallow: /shared
";

    assert_eq!(disallowed_paths(policy, "webproxy"), vec![
      "/private", "/shared"
    ]);
  }

  #[test]
  fn supports_multiple_agents_in_one_group() {
    let policy = "\
User-agent: archiver
User-agent: webproxy
Disallow: /
";

    assert_eq!(disallowed_paths(policy, "webproxy"), vec!["/"]);
  }

  #[test]
  fn ignores_empty_disallow_and_comments() {
    let policy = "\
USER-AGENT: webproxy
DISALLOW:
Disallow: /draft # This is private.
";

    assert_eq!(disallowed_paths(policy, "webproxy"), vec!["/draft"]);
  }

  #[test]
  fn empty_disallow_ends_a_group() {
    let policy = "\
User-agent: webproxy
Disallow:

User-agent: indexer
Disallow: /
";

    assert!(disallowed_paths(policy, "webproxy").is_empty());
  }
}
