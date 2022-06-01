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

use gmi::url::Url;

pub fn make(
  path: &str,
  fallback: bool,
  is_proxy: &mut bool,
) -> Result<Url, gmi::url::UrlParseError> {
  Ok(
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
    } else {
      // Try to set `ROOT` as `ROOT` environment variable, or use
      // `"gemini://fuwn.me"` as default.
      format!(
        "{}{}{}",
        {
          if let Ok(root) = std::env::var("ROOT") {
            root
          } else {
            warn!(
              "could not use ROOT from environment variables, proceeding with \
               default root: gemini://fuwn.me"
            );

            "gemini://fuwn.me".to_string()
          }
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
