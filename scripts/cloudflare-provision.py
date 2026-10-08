#!/usr/bin/env python3
"""Provision Cloudflare resources for a ConvexAutoBackup install.

Creates (or reuses, when they already exist):
  * an R2 bucket for offsite backups, plus an API token scoped to only that
    bucket (object read/write), converted to S3 credentials;
  * a remotely-managed Cloudflare Tunnel routing <hostname> to the app container;
  * a proxied DNS CNAME for <hostname>;
  * a Cloudflare Access application that only lets the listed emails in
    (one-time PIN login).

Authentication uses the account's global API key from the environment
(CLOUDFLARE_EMAIL + CLOUDFLARE_GLOBAL_API_KEY). The global key is only used
here; it is never written anywhere. Outputs:
  * TUNNEL_TOKEN merged into --env-file (mode 600)
  * R2 S3 credentials written as JSON to --r2-credentials (mode 600)

Example:
  CLOUDFLARE_EMAIL=... CLOUDFLARE_GLOBAL_API_KEY=... \\
    scripts/cloudflare-provision.py --account-id <id> --zone kodydennon.com \\
      --hostname dbr.kodydennon.com --bucket convex-autobackup-prod \\
      --allow-email you@example.com
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

API = "https://api.cloudflare.com/client/v4"
R2_ITEM_READ = "Workers R2 Storage Bucket Item Read"
R2_ITEM_WRITE = "Workers R2 Storage Bucket Item Write"


class CloudflareError(RuntimeError):
    pass


class Cloudflare:
    def __init__(self, email: str, key: str) -> None:
        self.headers = {
            "X-Auth-Email": email,
            "X-Auth-Key": key,
            "Content-Type": "application/json",
        }

    def call(self, method: str, path: str, body: object | None = None, query: dict | None = None):
        url = API + path
        if query:
            url += "?" + urllib.parse.urlencode(query)
        data = json.dumps(body).encode() if body is not None else None
        request = urllib.request.Request(url, data=data, method=method, headers=self.headers)
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                payload = json.load(response)
        except urllib.error.HTTPError as error:
            raw = error.read()
            try:
                payload = json.loads(raw)
            except ValueError:
                raise CloudflareError(f"{method} {path}: HTTP {error.code}: {raw[:300]!r}") from None
        if not payload.get("success", False):
            raise CloudflareError(f"{method} {path}: {payload.get('errors')}")
        return payload.get("result")


def log(message: str) -> None:
    print(message, flush=True)


def ensure_bucket(cf: Cloudflare, account: str, bucket: str) -> None:
    buckets = cf.call("GET", f"/accounts/{account}/r2/buckets")["buckets"]
    if any(item["name"] == bucket for item in buckets):
        log(f"R2 bucket {bucket}: exists")
        return
    cf.call("POST", f"/accounts/{account}/r2/buckets", {"name": bucket})
    log(f"R2 bucket {bucket}: created")


def ensure_r2_credentials(cf: Cloudflare, account: str, bucket: str, output: Path) -> None:
    if output.exists():
        existing = json.loads(output.read_text())
        try:
            token = cf.call("GET", f"/user/tokens/{existing['access_key_id']}")
            if token.get("status") == "active":
                log(f"R2 credentials: reusing active token {token['name']!r}")
                return
        except CloudflareError:
            pass
        log("R2 credentials: stored token is no longer active; creating a new one")

    groups = {group["name"]: group["id"] for group in cf.call("GET", "/user/tokens/permission_groups")}
    missing = [name for name in (R2_ITEM_READ, R2_ITEM_WRITE) if name not in groups]
    if missing:
        raise CloudflareError(f"permission groups not found: {missing}")
    name = f"convex-autobackup R2 {bucket}"
    for token in cf.call("GET", "/user/tokens", query={"per_page": 50}) or []:
        if token["name"] == name:
            cf.call("DELETE", f"/user/tokens/{token['id']}")
            log(f"R2 credentials: removed stale token {name!r}")
    token = cf.call(
        "POST",
        "/user/tokens",
        {
            "name": name,
            "policies": [
                {
                    "effect": "allow",
                    "resources": {f"com.cloudflare.edge.r2.bucket.{account}_default_{bucket}": "*"},
                    "permission_groups": [{"id": groups[R2_ITEM_READ]}, {"id": groups[R2_ITEM_WRITE]}],
                }
            ],
        },
    )
    credentials = {
        "account_id": account,
        "bucket": bucket,
        "endpoint": f"https://{account}.r2.cloudflarestorage.com",
        # R2 S3 credentials derive from an API token: id -> access key,
        # SHA-256 of the token value -> secret access key.
        "access_key_id": token["id"],
        "secret_access_key": hashlib.sha256(token["value"].encode()).hexdigest(),
    }
    write_private(output, json.dumps(credentials, indent=2) + "\n")
    log(f"R2 credentials: created token {name!r} scoped to bucket {bucket}; saved to {output}")


def ensure_tunnel(cf: Cloudflare, account: str, name: str, hostname: str, service: str) -> tuple[str, str]:
    tunnels = cf.call("GET", f"/accounts/{account}/cfd_tunnel", query={"name": name, "is_deleted": "false"}) or []
    if tunnels:
        tunnel_id = tunnels[0]["id"]
        log(f"Tunnel {name}: exists")
    else:
        tunnel_id = cf.call(
            "POST", f"/accounts/{account}/cfd_tunnel", {"name": name, "config_src": "cloudflare"}
        )["id"]
        log(f"Tunnel {name}: created")
    cf.call(
        "PUT",
        f"/accounts/{account}/cfd_tunnel/{tunnel_id}/configurations",
        {"config": {"ingress": [{"hostname": hostname, "service": service}, {"service": "http_status:404"}]}},
    )
    log(f"Tunnel {name}: routes {hostname} -> {service}")
    token = cf.call("GET", f"/accounts/{account}/cfd_tunnel/{tunnel_id}/token")
    return tunnel_id, token


def ensure_dns(cf: Cloudflare, zone_name: str, hostname: str, tunnel_id: str) -> None:
    zones = cf.call("GET", "/zones", query={"name": zone_name})
    if not zones:
        raise CloudflareError(f"zone {zone_name} not found")
    zone = zones[0]["id"]
    target = f"{tunnel_id}.cfargotunnel.com"
    record = {"type": "CNAME", "name": hostname, "content": target, "proxied": True, "comment": "ConvexAutoBackup tunnel"}
    existing = cf.call("GET", f"/zones/{zone}/dns_records", query={"name": hostname})
    if existing:
        current = existing[0]
        if current["type"] != "CNAME" or current["content"] != target or not current["proxied"]:
            if current["type"] != "CNAME" or not current["content"].endswith(".cfargotunnel.com"):
                raise CloudflareError(f"{hostname} already has an unrelated {current['type']} record; not overwriting")
            cf.call("PUT", f"/zones/{zone}/dns_records/{current['id']}", record)
            log(f"DNS {hostname}: updated to {target}")
        else:
            log(f"DNS {hostname}: exists")
        return
    cf.call("POST", f"/zones/{zone}/dns_records", record)
    log(f"DNS {hostname}: created CNAME -> {target}")


def ensure_access(cf: Cloudflare, account: str, hostname: str, emails: list[str]) -> None:
    idps = cf.call("GET", f"/accounts/{account}/access/identity_providers") or []
    otp = next((idp for idp in idps if idp["type"] == "onetimepin"), None)
    if otp is None:
        otp = cf.call(
            "POST",
            f"/accounts/{account}/access/identity_providers",
            {"name": "One-time PIN", "type": "onetimepin", "config": {}},
        )
        log("Access: created one-time PIN login method")
    app = {
        "name": "ConvexAutoBackup",
        "domain": hostname,
        "type": "self_hosted",
        "session_duration": "24h",
        "allowed_idps": [otp["id"]],
        "auto_redirect_to_identity": True,
        "app_launcher_visible": False,
        "policies": [
            {
                "name": "ConvexAutoBackup owners",
                "decision": "allow",
                "include": [{"email": {"email": email}} for email in emails],
                "precedence": 1,
            }
        ],
    }
    apps = cf.call("GET", f"/accounts/{account}/access/apps") or []
    existing = next((item for item in apps if item.get("domain") == hostname), None)
    if existing:
        cf.call("PUT", f"/accounts/{account}/access/apps/{existing['id']}", app)
        log(f"Access: updated app for {hostname} (allowed: {', '.join(emails)})")
    else:
        cf.call("POST", f"/accounts/{account}/access/apps", app)
        log(f"Access: created app for {hostname} (allowed: {', '.join(emails)})")


def write_private(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as handle:
        handle.write(content)
    os.chmod(path, 0o600)


def merge_env(path: Path, key: str, value: str) -> None:
    lines = path.read_text().splitlines() if path.exists() else []
    lines = [line for line in lines if not line.startswith(f"{key}=")]
    lines.append(f"{key}={value}")
    write_private(path, "\n".join(lines) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--account-id", required=True)
    parser.add_argument("--zone", required=True)
    parser.add_argument("--hostname", required=True)
    parser.add_argument("--bucket", required=True)
    parser.add_argument("--allow-email", action="append", required=True, dest="emails")
    parser.add_argument("--tunnel-name", default="convex-autobackup")
    parser.add_argument("--service", default="http://app:8976", help="origin the tunnel forwards to")
    parser.add_argument("--env-file", type=Path, default=Path.home() / ".convex-autobackup/docker.env")
    parser.add_argument("--r2-credentials", type=Path, default=Path.home() / ".convex-autobackup/r2-credentials.json")
    args = parser.parse_args()

    email = os.environ.get("CLOUDFLARE_EMAIL")
    key = os.environ.get("CLOUDFLARE_GLOBAL_API_KEY")
    if not email or not key:
        print("CLOUDFLARE_EMAIL and CLOUDFLARE_GLOBAL_API_KEY must be set", file=sys.stderr)
        return 2
    cf = Cloudflare(email, key)

    try:
        ensure_bucket(cf, args.account_id, args.bucket)
        ensure_r2_credentials(cf, args.account_id, args.bucket, args.r2_credentials)
        tunnel_id, tunnel_token = ensure_tunnel(cf, args.account_id, args.tunnel_name, args.hostname, args.service)
        ensure_dns(cf, args.zone, args.hostname, tunnel_id)
        ensure_access(cf, args.account_id, args.hostname, args.emails)
    except CloudflareError as error:
        print(f"Cloudflare provisioning failed: {error}", file=sys.stderr)
        return 1

    merge_env(args.env_file, "TUNNEL_TOKEN", tunnel_token)
    log(f"Tunnel token saved to {args.env_file}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
