use std::sync::LazyLock;

pub static ENVIRONMENT: LazyLock<Environment> =
  LazyLock::new(Environment::from_environment);

pub struct Environment {
  pub root:                       String,
  pub css_external:               Option<String>,
  pub primary_colour:             Option<String>,
  pub favicon_external:           Option<String>,
  pub mathjax:                    bool,
  pub head:                       Option<String>,
  pub header:                     Option<String>,
  pub plain_text_route:           Option<String>,
  pub condense_links:             Vec<String>,
  pub condense_links_at_headings: Vec<String>,
  pub proxy_by_default:           bool,
  pub keep_gemini:                Option<Vec<String>>,
  pub embed_images:               Option<String>,
  pub http09:                     bool,
  pub http09_port:                u16,
}

impl Environment {
  fn from_environment() -> Self {
    Self {
      root:                       std::env::var("ROOT").unwrap_or_else(|_| {
        log::warn!(
          "could not use ROOT from environment variables, proceeding with \
           default root: gemini://fuwn.me"
        );
        "gemini://fuwn.me".to_string()
      }),
      css_external:               std::env::var("CSS_EXTERNAL").ok(),
      primary_colour:             std::env::var("PRIMARY_COLOUR").ok(),
      favicon_external:           std::env::var("FAVICON_EXTERNAL").ok(),
      mathjax:                    std::env::var("MATHJAX")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(true),
      head:                       std::env::var("HEAD").ok(),
      header:                     std::env::var("HEADER").ok(),
      plain_text_route:           std::env::var("PLAIN_TEXT_ROUTE").ok(),
      condense_links:             std::env::var("CONDENSE_LINKS")
        .map(|s| s.split(',').map(String::from).collect())
        .unwrap_or_default(),
      condense_links_at_headings: std::env::var("CONDENSE_LINKS_AT_HEADINGS")
        .map(|s| s.split(',').map(String::from).collect())
        .unwrap_or_default(),
      proxy_by_default:           std::env::var("PROXY_BY_DEFAULT")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(true),
      keep_gemini:                std::env::var("KEEP_GEMINI")
        .ok()
        .map(|s| s.split(',').map(String::from).collect()),
      embed_images:               std::env::var("EMBED_IMAGES").ok(),
      http09:                     std::env::var("HTTP09")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false),
      http09_port:                std::env::var("HTTP09_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(90),
    }
  }
}
