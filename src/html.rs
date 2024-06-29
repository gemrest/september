use {germ::ast::Node, std::env::var, url::Url};

fn link_from_host_href(url: &Url, href: &str) -> Option<String> {
  Some(format!(
    "gemini://{}{}{}",
    url.domain()?,
    { if href.starts_with('/') { "" } else { "/" } },
    href
  ))
}

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn from_gemini(
  response: &germ::request::Response,
  url: &Url,
  is_proxy: bool,
) -> Option<(String, String)> {
  let ast_tree = germ::ast::Ast::from_string(
    response.content().as_ref().map_or_else(String::default, String::clone),
  );
  let ast = ast_tree.inner();
  let mut html = String::new();
  let mut title = String::new();
  let safe = html_escape::encode_text;
  let mut previous_link = false;
  let condense_links = {
    let links = var("CONDENSE_LINKS").map_or_else(
      |_| vec![],
      |condense_links| {
        condense_links
          .split(',')
          .map(std::string::ToString::to_string)
          .collect()
      },
    );

    links.contains(&url.path().to_string()) || links.contains(&"*".to_string())
  };
  let mut in_condense_links_flag_trap = true;
  let condensible_headings_value =
    var("CONDENSE_LINKS_AT_HEADINGS").unwrap_or_default();
  let condensible_headings =
    condensible_headings_value.split(',').collect::<Vec<_>>();

  for node in ast {
    if condensible_headings.contains(&node.to_gemtext().as_str()) {
      in_condense_links_flag_trap = true;
    }

    if previous_link
      && (!matches!(node, Node::Link { .. })
        || (!condense_links && !in_condense_links_flag_trap))
    {
      html.push_str("\n</p>");
      previous_link = false;
    } else if previous_link {
      html.push_str(" <span style=\"opacity: 50%;\">|</span> ");
    } else if !previous_link && matches!(node, Node::Link { .. }) {
      html.push_str("<p>");
    }

    match node {
      Node::Text(text) => html.push_str(&format!("<p>{}</p>", safe(text))),
      Node::Link { to, text } => {
        let mut href = to.to_string();
        let mut surface = false;

        if href.contains("://") && !href.starts_with("gemini://") {
          surface = true;
        } else if !href.starts_with("gemini://") && !href.starts_with('/') {
          href = format!(
            "{}/{}",
            url.domain().unwrap(),
            if url.path().ends_with('/') {
              format!("{}{}", url.path(), href)
            } else {
              format!("{}/{}", url.path(), href)
            }
          )
          .replace("//", "/");
          href = format!("gemini://{href}");
        } else if href.starts_with('/') || !href.contains("://") {
          href = link_from_host_href(url, &href)?;
        }

        if var("PROXY_BY_DEFAULT")
          .unwrap_or_else(|_| "true".to_string())
          .to_lowercase()
          == "true"
          && href.contains("gemini://")
          && !surface
        {
          if is_proxy
            || href
              .trim_start_matches("gemini://")
              .trim_end_matches('/')
              .split('/')
              .collect::<Vec<_>>()
              .first()
              .unwrap()
              != &url.host().unwrap().to_string().as_str()
          {
            href = format!("/proxy/{}", href.trim_start_matches("gemini://"));
          } else {
            href = href.trim_start_matches("gemini://").replacen(
              &if let Some(host) = url.host() {
                host.to_string()
              } else {
                return None;
              },
              "",
              1,
            );
          }
        }

        if let Ok(keeps) = var("KEEP_GEMINI_EXACT") {
          let mut keeps = keeps.split(',');

          if (href.starts_with('/') || !href.contains("://")) && !surface {
            let temporary_href = link_from_host_href(url, &href)?;

            if keeps.any(|k| k == &*temporary_href) {
              href = temporary_href;
            }
          }
        }

        if let Ok(keeps) = var("KEEP_GEMINI_DOMAIN") {
          let host = if let Some(host) = url.host() {
            host.to_string()
          } else {
            return None;
          };

          if (href.starts_with('/')
            || !href.contains("://") && keeps.split(',').any(|k| k == &*host))
            && !surface
          {
            href = link_from_host_href(url, &href)?;
          }
        }

        if let Ok(embed_images) = var("EMBED_IMAGES") {
          if let Some(extension) = std::path::Path::new(&href).extension() {
            if extension == "png"
              || extension == "jpg"
              || extension == "jpeg"
              || extension == "gif"
              || extension == "webp"
              || extension == "svg"
            {
              if embed_images == "1" {
                html.push_str(&format!(
                  "<p><a href=\"{}\">{}</a> <i>Embedded below</i></p>\n",
                  href,
                  safe(text.as_ref().unwrap_or(to)),
                ));
              }

              html.push_str(&format!(
                "<p><img src=\"{}\" alt=\"{}\" /></p>\n",
                safe(&href),
                safe(text.as_ref().unwrap_or(to)),
              ));

              continue;
            }
          }
        }

        previous_link = true;

        html.push_str(&format!(
          "<a href=\"{}\">{}</a>",
          href,
          safe(text.as_ref().unwrap_or(to)),
        ));
      }
      Node::Heading { level, text } => {
        if !condensible_headings.contains(&node.to_gemtext().as_str()) {
          in_condense_links_flag_trap = false;
        }

        if title.is_empty() && *level == 1 {
          title = safe(text).to_string();
        }

        html.push_str(&format!(
          "<{}>{}</{0}>",
          match level {
            1 => "h1",
            2 => "h2",
            3 => "h3",
            _ => "p",
          },
          safe(text),
        ));
      }
      Node::List(items) => html.push_str(&format!(
        "<ul>{}</ul>",
        items
          .iter()
          .map(|i| format!("<li>{i}</li>"))
          .collect::<Vec<String>>()
          .join("\n")
      )),
      Node::Blockquote(text) => {
        html.push_str(&format!("<blockquote>{}</blockquote>", safe(text)));
      }
      Node::PreformattedText { text, .. } => {
        html.push_str(&format!("<pre>{}</pre>", safe(text)));
      }
      Node::Whitespace => {}
    }
  }

  Some((title, html))
}
