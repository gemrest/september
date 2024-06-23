# September

[![github.com](https://github.com/gemrest/september/actions/workflows/check.yaml/badge.svg?branch=main)](https://github.com/gemrest/september/actions/workflows/check.yaml)

September is a simple and efficient Gemini-to-HTTP proxy written in [Rust](https://www.rust-lang.org/).

September remains simple, but packs more features than you could imagine, all configurable via environment variables.

## Usage

A production deployment of September can be found at https://fuwn.me, with the root capsule set as [gemini://fuwn.me](gemini://fuwn.me).

You can try proxying any external capsule through the `/proxy/` route: https://fuwn.me/proxy/geminiprotocol.net/.

### Docker

`docker run` allows you to pass environment variables via the `-e` flag.

```shell
# September with a custom root, listening on port 8080
docker run -d \
  -e ROOT="gemini://fuwn.me" \
  -p 8080:80 \
  fuwn/september:latest

# September with a custom root, port, and external stylesheet, listening on port 80
docker run -d \
  -e ROOT="gemini://fuwn.me" \
  -e PORT="8080" \
  -e CSS_EXTERNAL="https://example.com/style.css" \
  -p 80:80 \
  fuwn/september:latest
```

You may start to find this way of passing configuration cumbersome for many options, so Docker management tool like [Portainer](https://www.portainer.io/) or a Docker Compose file might come in handy.

### Docker Compose

Docker Compose is a file-configurable Docker utility to make deploying exact container configuration and configuration sets simple. This repository provides a sample Docker Compose file, [`./docker-compose.yaml`](./docker-compose.yaml), with some examples configuration values that you can modify to your liking.

After editing the file, you can bring up the composition using `docker-compose` command.

```shell
docker-compose up -d
```

### Executable

While generally discouraged, you can run the September executable by itself and configure it through environment variables.

```shell
ROOT="gemini://fuwn.me" PORT="8080" CSS_EXTERNAL="https://example.com/style.css" ./september
```

If available, September will use the relative directory's `.env` file for populating its configuration. Here is an example `.env` file with a few values added.

```dotenv
# .env

ROOT=gemini://fuwn.me
PORT=8080
CSS_EXTERNAL=https://example.com/style.css
HEAD=<script>/* This will appear in the head of the HTML document. */</script>
```

## Configuration

All configuration options with examples can be found in the [Configuration.md](./Configuration.md) file. Regardless of deployment method, these options remain present in each case.

## Styling

Want to give your website a shiny new look? Try using one of these sources to find a stylish and **minimal** (!!) CSS theme/ framework!

- [dohliam/dropin-minimal-css](https://github.com/dohliam/dropin-minimal-css): Drop-in switcher for previewing minimal CSS frameworks
- [dbohdan/classless-css](https://github.com/dbohdan/classless-css): A list of classless CSS themes/frameworks with screenshots

## Origins

The story of September starts with a simple request to add environment variable-configurable options to a pre-existing Gemini proxy.

The proxy in question already had options, just that they were command-line flag configurable options. Apparently, containerising a networked application is not a "valid use-case", and everyone should prefer running raw binaries on their systems and servers. Also, finicky command-line arguments reign superior to the industry standard environment variable, or at least that's what I gather from this author's response to adding a few extra lines of code that I already wrote out for environment variable support.

Anyway, I forked the proxy. Somewhere down the line, I realised that this proxy just isn't cutting it and was poorly designed to begin with, so I threw it in the figurative trash, and wrote September from scratch.

In the end, it all worked out, since September has become the easiest to configure, most feature-packed, quickest to understand (and quickest in general) Gemini-to-HTTP proxy of the bunch.

## License

This project is licensed with the [GNU General Public License v3.0](./LICENSE).
