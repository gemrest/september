// This file is part of September <https://github.com/gemrest/september>.
// Copyright (C) 2022-2022 Fuwn <contact@fuwn.me>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Copyright (C) 2022-2022 Fuwn <contact@fuwn.me>
// SPDX-License-Identifier: GPL-3.0-only

use std::env::var;

use germ::ast::Node;
use gmi::url::Url;
use markly::to_html;

fn link_from_host_href(url: &Url, href: &str) -> String {
  format!(
    "gemini://{}{}{}",
    url.authority.host,
    {
      if href.starts_with('/') {
        ""
      } else {
        "/"
      }
    },
    href
  )
}

#[allow(clippy::too_many_lines)]
pub fn gemini_to_html(
  response: &gmi::protocol::Response,
  url: &Url,
  is_proxy: bool,
) -> (String, String) {
  let ast_tree =
    germ::ast::Ast::from_string(&String::from_utf8_lossy(&response.data));
  let ast = ast_tree.inner();
  let mut html = String::new();
  let mut title = String::new();

  for node in ast {
    match node {
      Node::Text(text) => html.push_str(&format!("<p>{}</p>", to_html(text))),
      Node::Link { to, text } => {
        let mut href = to.clone();
        let mut surface = false;

        if href.contains("://") && !href.starts_with("gemini://") {
          surface = true;
        } else if !href.starts_with("gemini://") && !href.starts_with('/') {
          href = format!("./{href}");
        } else if href.starts_with('/') || !href.contains("://") {
          href = link_from_host_href(url, &href);
        }

        if var("PROXY_BY_DEFAULT").unwrap_or_else(|_| "true".to_string())
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
              != &url.authority.host.as_str()
          {
            href = format!("/proxy/{}", href.trim_start_matches("gemini://"));
          } else {
            href = href
              .trim_start_matches("gemini://")
              .replace(&url.authority.host, "");
          }
        }

        if let Ok(keeps) = var("KEEP_GEMINI_EXACT") {
          let mut keeps = keeps.split(',');

          if (href.starts_with('/') || !href.contains("://")) && !surface {
            let temporary_href = link_from_host_href(url, &href);

            if keeps.any(|k| k == &*temporary_href) {
              href = temporary_href;
            }
          }
        }

        if let Ok(keeps) = var("KEEP_GEMINI_DOMAIN") {
          if (href.starts_with('/')
            || !href.contains("://")
              && keeps.split(',').any(|k| k == &*url.authority.host))
            && !surface
          {
            href = link_from_host_href(url, &href);
          }
        }

        html.push_str(&format!(
          "<p><a href=\"{}\">{}</a></p>\n",
          href,
          to_html(&text.clone().unwrap_or_default())
        ));
      }
      Node::Heading { level, text } => {
        if title.is_empty() && *level == 1 {
          title = text.clone();
        }

        html.push_str(&format!(
          "<{}>{}</{0}>",
          match level {
            1 => "h1",
            2 => "h2",
            3 => "h3",
            _ => "p",
          },
          to_html(text)
        ));
      }
      Node::List(items) => html.push_str(&format!(
        "<ul>{}</ul>",
        items
          .iter()
          .map(|i| format!("<li>{}</li>", to_html(i)))
          .collect::<Vec<String>>()
          .join("\n")
      )),
      Node::Blockquote(text) => {
        html.push_str(&format!("<blockquote>{}</blockquote>", to_html(text)));
      }
      Node::PreformattedText { text, .. } => {
        html.push_str(&format!("<pre>{text}</pre>"));
      }
      Node::Whitespace => {}
    }
  }

  (title, html)
}
