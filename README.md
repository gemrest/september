# September

[![github.com](https://github.com/gemrest/september/actions/workflows/check.yaml/badge.svg?branch=main)](https://github.com/gemrest/september/actions/workflows/check.yaml)

A simple and efficient Gemini-to-HTTP proxy written in Rust.

## Usage

### Docker

```shell
$ docker run -d [ -e ROOT="gemini://fuwn.me" ] [ -e PORT="8080"] [ -e CSS_EXTERNAL="https://example.com/style.css"] fuwn/september:latest
```

### Docker Compose

Edit the `docker-compose.yaml` file to your liking, and then

```shell
$ docker-compose up -d
```

### Executable

```shell
$ [ ROOT="gemini://fuwn.me" ] [ PORT="8080"] [ CSS_EXTERNAL="https://example.com/style.css"] ./september
```

or use a `.env` file

```dotenv
# .env

ROOT=gemini://fuwn.me
PORT=8080
CSS_EXTERNAL=https://example.com/style.css
```

and then

```shell
$ ./september
```

## Configuration

Configuration for September is done solely via environment variables, for
simplicity, and Docker support.

### `PORT`

Bind September to a custom port.

Generally, you shouldn't touch this if you are deploying using Docker.

If no `PORT` is provided or the `PORT` could not be properly parsed as a `u16`;
port `80` will be assumed.

```dotenv
PORT=8080
```

### `ROOT`

The root Gemini capsule to proxy when not visiting a "/proxy" route.

If no `ROOT` is provided, `"gemini://fuwn.me"` will be assumed.

```dotenv
ROOT=gemini://fuwn.me
```

### `CSS_EXTERNAL`

An external CSS file to apply to the HTML response.

If no `CSS_EXTERNAL` is provided, there will be no styling done to the HTML
response.

```dotenv
CSS_EXTERNAL=https://cdnjs.cloudflare.com/ajax/libs/mini.css/3.0.1/mini-default.min.css
```

### `KEEP_GEMINI_EXACT`

Keeps exactly matching URLs as a Gemini URL.

#### Examples

If `KEEP_GEMINI_EXACT` is equal to `KEEP_GEMINI_EXACT=gemini://fuwn.me/gemini`,
all routes will be proxied their "/proxy" equivalent (e.g.,
"https://fuwn.me/proxy/fuwn.me/gopher"), except occurrences of
"gemini://fuwn.me/skills" will be kept as is.

```dotenv
KEEP_GEMINI_EXACT=gemini://fuwn.me/skills
```

### `KEEP_GEMINI_DOMAIN`

Similar to `KEEP_GEMINI_EXACT`, except proxies based on entire domains instead
of exact matches.

```dotenv
KEEP_GEMINI_DOMAIN=fuwn.me
```

### `PROXY_BY_DEFAULT`

Control weather or not all Gemini URLs will be proxied.

Similar to `KEEP_GEMINI_EXACT` and `KEEP_GEMINI_DOMAIN` but global.

Defaults to `true`.

```dotenv
PROXY_BY_DEFAULT=false
```

### `FAVICON_EXTERNAL`

An external favicon file to apply to the HTML response.

```dotenv
FAVICON_EXTERNAL=https://host.fuwn.me/8te8lw0lxm03.webp
```

### `PLAIN_TEXT_ROUTE`

A comma-seperated list of paths to treat as plain text routes.

```dotenv
PLAIN_TEXT_ROUTE=/robots.txt,/license.txt
```

## Styling

Want to give your website a shiny new look? Try using one of sources
to find a stylish and **minimal** (!!) CSS theme/ framework!

- [dohliam/dropin-minimal-css](https://github.com/dohliam/dropin-minimal-css): Drop-in switcher for previewing minimal CSS frameworks
- [dbohdan/classless-css](https://github.com/dbohdan/classless-css): A list of classless CSS themes/frameworks with screenshots

## Capsules using September

[Add yours!](https://github.com/gemrest/september/edit/main/README.md)

- <https://fuwn.me/>
- <https://gem.rest/>

## License

This project is licensed with the
[GNU General Public License v3.0](https://github.com/gemrest/september/blob/main/LICENSE).
