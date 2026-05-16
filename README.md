![example workflow](https://github.com/d1ngd0/kla/actions/workflows/test.yaml/badge.svg)

# KLA

Kla is a cli tool for interacting with HTTP restful interfaces. The intent is to be a bridge between that interface and your terminal. Kla is actively in development, but could be used to replace curl in your normal routine. 

Some Examples to give you an idea

Run a `GET` request against an elasticsearch cluster to see the nodes:

```bash
kla --env my-es-cluster cat/nodes
```

Delete a user from your CEPH Cluster through the Radoswg-admin

```bash
kla --sigv4 --env ceph-prod delete '/admin/user?uid=terry'
# OR
kla --sigv4 --env ceph-prod delete /admin/user --query uid=terry
```

Run a `POST` method on your ntfy cluster:

```
kla --env ntfy post mytopic 'hello world'
```

You can switch between environments with `kla switch` and run commands without `--env`

For things you do often you can create a [[template]]

```
kla --env poetry run authors 
```

# Installing

There are a few different ways to install kla onto your machine

## Latest Releases

Check out our [latest release](https://github.com/d1ngd0/kla/releases/latest) for a pre-built binary. Linux and mac supported.

```bash
curl https://github.com/d1ngd0/kla/releases/download/0.0.3/kla-aarch64-apple-0.0.3 -o kla
chmod +x kla
sudo mv kla /usr/local/bin/kla
```

## By Source Installation

The following assume you `cargo` installed

```bash
# Clone the repo
git clone https://github.com/d1ngd0/kla
# cd into the directory
cd kla
# Install it
just install
```
