#!/bin/sh
# Independent official bootstrap. Generated with the audited release verifier and roots.
set -eu
PATH=/usr/sbin:/usr/bin:/sbin:/bin:/usr/local/sbin:/usr/local/bin
export PATH
umask 077

if [ "${1:-}" = --help ]; then
  printf '%s\n' '用法: bootstrap.sh --panel https://面板 --token 接入令牌 --version latest或版本 --target auto或平台'
  exit 0
fi
if [ "$(id -u)" != 0 ]; then
  echo '请以 root 执行此脚本，或使用面板提供的官方单行安装入口。' >&2
  exit 1
fi
PLATFORM=$(uname -s)
case "$PLATFORM" in
  Linux)
    STAGING_BASE=/opt/sinan
    if [ -d /run/systemd/system ] && command -v systemctl >/dev/null; then :
    elif [ -f /run/openrc/softlevel ] && command -v rc-service >/dev/null && command -v rc-update >/dev/null; then :
    else echo 'Linux 接入需要运行中的 systemd 或 OpenRC' >&2; exit 1; fi
    ;;
  Darwin)
    [ "$(uname -m)" = arm64 ] || { echo 'macOS Agent 仅支持 ARM64' >&2; exit 1; }
    STAGING_BASE=/opt/sinan
    ;;
  FreeBSD) STAGING_BASE=/opt/sinan ;;
  *) echo '此入口支持 Linux、macOS 和 FreeBSD；Windows 请使用 PowerShell 入口' >&2; exit 1 ;;
esac
case "$(uname -m)" in x86_64|amd64|aarch64|arm64) ;; *) echo '此接入入口仅支持 AMD64 和 ARM64' >&2; exit 1 ;; esac

# /run and system temporary volumes can be noexec; /opt is the Agent executable volume.
protected_directory() {
  PROTECTED_CHECK=$1
  while :; do
    [ -d "$PROTECTED_CHECK" ] && [ ! -L "$PROTECTED_CHECK" ] || { echo '安装临时目录与父目录必须是普通目录' >&2; exit 1; }
    if [ "$PLATFORM" = Linux ]; then
      PROTECTED_OWNER=$(stat -c '%u' "$PROTECTED_CHECK")
      PROTECTED_MODE=$(stat -c '%a' "$PROTECTED_CHECK")
    else
      PROTECTED_OWNER=$(/usr/bin/stat -f '%u' "$PROTECTED_CHECK")
      PROTECTED_MODE=$(/usr/bin/stat -f '%Lp' "$PROTECTED_CHECK")
    fi
    [ "$PROTECTED_OWNER" = 0 ] && [ "$((0$PROTECTED_MODE & 022))" = 0 ] || { echo '安装临时目录与父目录必须由 root 保护，不能由其他账户写入' >&2; exit 1; }
    [ "$PROTECTED_CHECK" != / ] || break
    PROTECTED_CHECK=$(dirname "$PROTECTED_CHECK")
  done
}
EXISTING_PARENT=$STAGING_BASE
while [ ! -e "$EXISTING_PARENT" ] && [ ! -L "$EXISTING_PARENT" ]; do EXISTING_PARENT=$(dirname "$EXISTING_PARENT"); done
protected_directory "$EXISTING_PARENT"
(umask 022; mkdir -p "$STAGING_BASE")
protected_directory "$STAGING_BASE"
STAGING=$(mktemp -d "$STAGING_BASE/sinan-bootstrap.XXXXXX")
cleanup() { rm -rf "$STAGING"; }
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
PYTHON=python3
MINISIGN=minisign

if [ "$PLATFORM" = Darwin ]; then
  # Use independently pinned upstream tools instead of running Homebrew as root.
  PYTHON=/Library/Frameworks/Python.framework/Versions/3.13/bin/python3.13
  if [ ! -x "$PYTHON" ]; then
    /usr/bin/curl --fail --silent --show-error --proto '=https' --noproxy '*' --connect-timeout 20 --max-time 600 --max-filesize 100000000 https://www.python.org/ftp/python/3.13.16/python-3.13.16-macos11.pkg -o "$STAGING/python.pkg"
    printf '%s  %s\n' 30666509020b4da0dd8bc2e773255f34d76b7bb80b66960a928d5f6daa0192d7 "$STAGING/python.pkg" | /usr/bin/shasum -a 256 -c - >/dev/null
    /usr/sbin/pkgutil --check-signature "$STAGING/python.pkg" >/dev/null
    /usr/sbin/installer -pkg "$STAGING/python.pkg" -target /
  fi
  # Resolve the fixed official Framework alias, then execute only its protected real path.
  PYTHON_LINKS=0
  while [ -L "$PYTHON" ]; do
    PYTHON_LINKS=$((PYTHON_LINKS + 1))
    [ "$PYTHON_LINKS" -le 16 ] || { echo 'Python Framework 链接过多' >&2; exit 1; }
    PYTHON_PARENT=$(dirname "$PYTHON")
    PYTHON_LINK=$(/usr/bin/readlink "$PYTHON")
    case "$PYTHON_LINK" in /*) PYTHON=$PYTHON_LINK ;; *) PYTHON="$PYTHON_PARENT/$PYTHON_LINK" ;; esac
  done
  PYTHON_PARENT=$(cd -P "$(dirname "$PYTHON")" && pwd -P)
  PYTHON="$PYTHON_PARENT/$(basename "$PYTHON")"
  PYTHON_CHECK=$PYTHON
  while :; do
    PYTHON_OWNER=$(/usr/bin/stat -f '%u' "$PYTHON_CHECK")
    PYTHON_MODE=$(/usr/bin/stat -f '%Lp' "$PYTHON_CHECK")
    [ "$PYTHON_OWNER" = 0 ] && [ "$((0$PYTHON_MODE & 022))" = 0 ] && [ ! -L "$PYTHON_CHECK" ] || { echo 'Python Framework 与父目录必须由 root 保护' >&2; exit 1; }
    [ "$PYTHON_CHECK" != / ] || break
    PYTHON_CHECK=$(dirname "$PYTHON_CHECK")
  done
  /usr/bin/curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --noproxy '*' --connect-timeout 20 --max-time 120 --max-filesize 1000000 https://github.com/jedisct1/minisign/releases/download/0.12/minisign-0.12-macos.zip -o "$STAGING/minisign.zip"
  printf '%s  %s\n' 89000b19535765f9cffc65a65d64a820f433ef6db8020667f7570e06bf6aac63 "$STAGING/minisign.zip" | /usr/bin/shasum -a 256 -c - >/dev/null
  "$PYTHON" -I - "$STAGING/minisign.zip" "$STAGING/minisign" <<'PY'
import hashlib, pathlib, stat, sys, zipfile
with zipfile.ZipFile(sys.argv[1]) as archive:
    if set(archive.namelist()) != {'minisign', '._minisign'}:
        raise SystemExit('minisign 官方归档包含非预期文件')
    member = archive.getinfo('minisign')
    if member.file_size != 180736 or member.is_dir() or stat.S_ISLNK(member.external_attr >> 16):
        raise SystemExit('minisign 官方归档文件无效')
    data = archive.read(member)
    if hashlib.sha256(data).hexdigest() != 'd41cde458303d45c95b00473e2455a7f45f95b550931f1f0cc98ef1f61b2a8ff':
        raise SystemExit('minisign 官方可执行文件摘要不匹配')
    target = pathlib.Path(sys.argv[2])
    target.write_bytes(data)
    target.chmod(0o700)
PY
  MINISIGN="$STAGING/minisign"
  /usr/bin/security find-certificate -a -p /System/Library/Keychains/SystemRootCertificates.keychain > "$STAGING/system-ca.pem"
  [ -s "$STAGING/system-ca.pem" ] || { echo '无法读取 macOS 系统信任根证书' >&2; exit 1; }
  SSL_CERT_FILE="$STAGING/system-ca.pem"
  export SSL_CERT_FILE
else

NEEDS_PACKAGES=0
TOOLS='python3'
if [ "$PLATFORM" = Linux ]; then
  TOOLS="$TOOLS install getent cmp mv seq"
  mv --version >/dev/null 2>&1 || NEEDS_PACKAGES=1
else
  TOOLS="$TOOLS minisign"
fi
for tool in $TOOLS; do
  command -v "$tool" >/dev/null || NEEDS_PACKAGES=1
done
if [ "$NEEDS_PACKAGES" = 1 ]; then
  printf '%s\n' '正在通过系统软件源准备 Python、minisign 和安装工具。'
  if command -v apt-get >/dev/null; then
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends python3 ca-certificates coreutils libc-bin passwd
  elif command -v apk >/dev/null; then
    apk add --no-cache python3 ca-certificates coreutils musl-utils shadow
  elif command -v dnf >/dev/null; then
    dnf install -y python3 ca-certificates coreutils glibc-common shadow-utils
  elif command -v yum >/dev/null; then
    yum install -y python3 ca-certificates coreutils glibc-common shadow-utils
  elif [ "$PLATFORM" = FreeBSD ] && command -v pkg >/dev/null; then
    ASSUME_ALWAYS_YES=yes pkg bootstrap -f
    pkg install -y python3 minisign ca_root_nss
  else
    echo '无法自动准备安装工具：需要系统 apt-get、apk、dnf、yum 或 FreeBSD pkg 软件源' >&2
    exit 1
  fi
fi
for tool in $TOOLS; do
  command -v "$tool" >/dev/null || { echo "系统软件源未提供所需工具: $tool" >&2; exit 1; }
done
fi
cat > "$STAGING/bootstrap.py" <<'SINAN_BOOTSTRAP_7415F3BDC30846B6CEC2230C27EC5FF57840ABA1FA0BC3BCA2E47C30DA90269B'
#!/usr/bin/env python3
"""Trusted, operator-provisioned bootstrap; never fetched from the panel and executed."""

import argparse
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import tarfile
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

from legacy_agent_checkpoint import preflight as legacy_checkpoint_preflight

from release import (REPOSITORY, VERSION, digest, ensure, load_roots, read_regular,
                     require_protected_file, validate_manifest, verify_manifest)

GITHUB_DOWNLOAD_HOSTS = frozenset(("github.com", "release-assets.githubusercontent.com",
                                    "objects.githubusercontent.com"))
PROTOCOL_VERSION = 1
MINISIGN_LINUX_ARCHIVE_SHA256 = "9a599b48ba6eb7b1e80f12f36b94ceca7c00b7a5173c95c3efc88d9822957e73"
MINISIGN_LINUX_BINARIES = {
    "x86_64": (288200, "2c74dffcc1c9a5ee55957c60971998ace2b89f22585631594ec2152c588af8db"),
    "aarch64": (195288, "cec9f88be8c975af76854a53b4d49c3d257feae38d916edb0d16fb55aacd3000"),
}
PROOF_FILES = (("SHA256SUMS", 8192), ("SHA256SUMS.minisig", 16384),
               ("release.json", 32768), ("install.sh", 262144))
PRELOADED_INSTALLER_MARKER = b"# SINAN_BOOTSTRAP_AGENT_SOURCE=preloaded-github-v1"
DOWNLOAD_BUDGET_SECONDS = 300
DOWNLOAD_SOCKET_TIMEOUT = 20
CATALOG_BUDGET_SECONDS = 30


class IncompatibleRelease(ValueError):
    pass


def require_preloaded_installer(installer):
    """Check the actual independently trusted executor, not an unexecuted release file."""
    content = read_regular(Path(installer), 262144)
    ensure(content.splitlines().count(PRELOADED_INSTALLER_MARKER) == 1,
           "trusted Linux installer requires the preloaded-GitHub Agent contract")


def bounded_read(response, size, deadline, message="GitHub download exceeded total time budget"):
    remaining = deadline - time.monotonic()
    ensure(remaining > 0, message)
    # urllib otherwise renews its timeout for every socket read. Bound each
    # active HTTP(S) read by this file's remaining total budget as well.
    sock = getattr(getattr(getattr(response, "fp", None), "raw", None), "_sock", None)
    if sock is not None:
        sock.settimeout(min(DOWNLOAD_SOCKET_TIMEOUT, remaining))
    read = getattr(response, "read1", response.read)
    block = read(size)
    ensure(time.monotonic() < deadline, message)
    return block


def validate_mirror(value, panel=None):
    if not value:
        return ""
    parsed = urllib.parse.urlsplit(value)
    ensure(len(value) <= 512 and not any(ord(c) < 32 or ord(c) == 127 for c in value)
           and parsed.scheme == "https" and parsed.hostname and parsed.hostname != "localhost"
           and parsed.port in (None, 443) and not parsed.username and not parsed.password
           and not parsed.query and not parsed.fragment, "invalid HTTPS mirror prefix")
    try:
        ipaddress.ip_address(parsed.hostname)
    except ValueError:
        pass
    else:
        raise ValueError("mirror must be a hostname")
    if panel:
        origin = urllib.parse.urlsplit(panel)
        ensure((parsed.hostname, parsed.port or 443) != (origin.hostname, origin.port or (443 if origin.scheme == "https" else 80)), "Agent cannot be downloaded from panel")
    return value.rstrip("/")


def validate_github_url(url, mirror=""):

    parsed = urllib.parse.urlsplit(url)
    ensure(not any(ord(character) < 32 or ord(character) == 127 for character in url)
           and parsed.scheme == "https" and (parsed.hostname in GITHUB_DOWNLOAD_HOSTS or
               (mirror and parsed.netloc == urllib.parse.urlsplit(mirror).netloc))
           and parsed.port in (None, 443) and not parsed.username and not parsed.password
           and not parsed.fragment, "GitHub download URL is outside the fixed HTTPS allowlist")


def validate_panel_origin(value):
    parsed = urllib.parse.urlsplit(value)
    ensure(not any(ord(character) < 32 or ord(character) == 127 for character in value)
           and parsed.scheme in ("http", "https") and parsed.hostname
           and not parsed.username and not parsed.password and parsed.path in ("", "/")
           and not parsed.query and not parsed.fragment
           and (parsed.port is None or 0 < parsed.port <= 65535), "panel must be a valid origin")
    if parsed.scheme == "http":
        try:
            address = ipaddress.ip_address(parsed.hostname)
            address = getattr(address, "ipv4_mapped", None) or address
            loopback = address.is_loopback
        except ValueError:
            loopback = parsed.hostname == "localhost"
        ensure(loopback, "HTTP panel origins must use a loopback address; use HTTPS")


class GithubRedirect(urllib.request.HTTPRedirectHandler):
    max_redirections = 5
    max_repeats = 2

    def __init__(self, mirror=""):
        super().__init__()
        self.mirror = mirror

    def redirect_request(self, request, response, code, message, headers, new_url):
        validate_github_url(new_url, self.mirror)
        return super().redirect_request(request, response, code, message, headers, new_url)


def github_opener(mirror=""):
    # Bootstrap proof downloads never inherit HTTP_PROXY/HTTPS_PROXY/ALL_PROXY.
    return urllib.request.build_opener(urllib.request.ProxyHandler({}), GithubRedirect(mirror))


def download(base, name, destination, limit, mirror=""):
    ensure(Path(name).name == name and name not in (".", ".."), "unsafe asset name")
    url = base + "/" + urllib.parse.quote(name, safe="")
    mirror = validate_mirror(mirror)
    if mirror:
        url = mirror + "/" + url
    validate_github_url(url, mirror)
    deadline = time.monotonic() + DOWNLOAD_BUDGET_SECONDS
    with github_opener(mirror).open(url, timeout=DOWNLOAD_SOCKET_TIMEOUT) as response:
        validate_github_url(response.url, mirror)
        blocks, total = [], 0
        while True:
            block = bounded_read(response, min(65536, limit + 1 - total), deadline)
            if not block:
                break
            total += len(block)
            ensure(total <= limit, "download size outside permitted range")
            blocks.append(block)
        data = b"".join(blocks)
    ensure(0 < len(data) <= limit, "download size outside permitted range")
    destination.write_bytes(data)


def prepare_linux_minisign(staging):
    ensure(platform.system() == "Linux", "minisign 静态后备包仅适用于 Linux")
    machine = {"amd64": "x86_64", "arm64": "aarch64"}.get(platform.machine(), platform.machine())
    ensure(machine in MINISIGN_LINUX_BINARIES, "minisign 不支持此 CPU")
    staging = Path(staging)
    require_protected_file(staging)
    archive = staging / "minisign.tar.gz"
    download("https://github.com/jedisct1/minisign/releases/download/0.12",
             "minisign-0.12-linux.tar.gz", archive, 1048576)
    ensure(hashlib.sha256(archive.read_bytes()).hexdigest() == MINISIGN_LINUX_ARCHIVE_SHA256,
           "minisign 官方后备归档摘要不匹配")
    name = "minisign-linux/" + machine + "/minisign"
    size, checksum = MINISIGN_LINUX_BINARIES[machine]
    with tarfile.open(archive, "r:gz") as source:
        matches = [member for member in source if member.name == name]
        ensure(len(matches) == 1 and matches[0].isfile() and matches[0].size == size,
               "minisign 官方后备包缺少唯一目标 CPU 文件")
        binary = source.extractfile(matches[0]).read(size + 1)
    ensure(len(binary) == size and hashlib.sha256(binary).hexdigest() == checksum,
           "minisign 官方后备可执行文件摘要不匹配")
    target = staging / "minisign"
    target.write_bytes(binary)
    target.chmod(0o700)
    return str(target)


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, new_url):
        raise ValueError("面板响应禁止重定向")


def panel_opener():
    return urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())


def host_target():
    arch = {"x86_64": "amd64", "amd64": "amd64", "aarch64": "arm64", "arm64": "arm64"}.get(platform.machine())
    ensure(arch is not None, "仅支持 AMD64 和 ARM64 架构")
    system = platform.system()
    if system == "Darwin":
        ensure(arch == "arm64", "macOS Agent 仅支持 ARM64")
        return "macos-arm64"
    if system == "FreeBSD":
        return "freebsd-" + arch
    ensure(system == "Linux", "此入口支持 Linux、macOS 和 FreeBSD；Windows 请使用 PowerShell 入口")
    libc = platform.libc_ver()[0].lower()
    if libc in ("glibc", "gnu"):
        return "linux-gnu-" + arch
    if libc == "musl" or list(Path("/lib").glob("ld-musl-*.so.1")):
        return "linux-musl-" + arch
    try:
        result = subprocess.run(["ldd", "--version"], capture_output=True, check=False, timeout=10)
        description = (result.stdout + result.stderr).decode("utf-8", errors="replace").lower()
    except (OSError, subprocess.TimeoutExpired):
        description = ""
    if "musl" in description:
        return "linux-musl-" + arch
    if "glibc" in description or "gnu libc" in description:
        return "linux-gnu-" + arch
    raise ValueError("无法识别 Linux libc，拒绝猜测 GNU/musl 制品")


def compatible_targets(actual, requested="auto"):
    if actual.startswith("linux-"):
        arch = actual.rsplit("-", 1)[1]
        targets = ["linux-musl-" + arch, arch]
        if actual.startswith("linux-gnu-"):
            targets.append(actual)
    else:
        targets = [actual]
    ensure(requested == "auto" or requested in targets,
           f"所选平台 {requested} 与本机 {actual} 不兼容，请选择自动匹配或本机平台")
    if requested != "auto":
        targets.remove(requested)
        targets.insert(0, requested)
    return targets


def catalog(panel, token, target, version):
    validate_panel_origin(panel)
    parameters = {"token": token, "target": target}
    if version != "latest":
        parameters["agent_version"] = version
    url = panel.rstrip("/") + "/api/bootstrap/versions?" + urllib.parse.urlencode(parameters)
    deadline = time.monotonic() + CATALOG_BUDGET_SECONDS
    try:
        with panel_opener().open(url, timeout=CATALOG_BUDGET_SECONDS) as response:
            ensure(response.status == 200 and response.url == url, "接入版本目录响应无效")
            blocks, total = [], 0
            while True:
                block = bounded_read(response, min(65536, 131073 - total), deadline,
                                     "panel catalog download exceeded total time budget")
                if not block:
                    break
                total += len(block)
                ensure(total <= 131072, "接入版本目录超出大小限制")
                blocks.append(block)
            encoded = b"".join(blocks)
    except urllib.error.HTTPError as error:
        raise ValueError("无法获取接入版本，请检查令牌是否有效以及面板是否已导入签名 Release") from error
    ensure(0 < len(encoded) <= 131072, "接入版本目录超出大小限制")
    value = json.loads(encoded)
    ensure(isinstance(value, dict) and isinstance(value.get("versions"), list)
           and len(value["versions"]) <= 256, "接入版本目录格式无效")
    candidates = []
    seen = set()
    for item in value["versions"]:
        ensure(isinstance(item, dict), "接入版本条目无效")
        release_version = item.get("version")
        ensure(isinstance(release_version, str) and VERSION.fullmatch(release_version)
               and item.get("tag") == "agent-v" + release_version
               and isinstance(item.get("targets"), list)
               and all(isinstance(t, str) for t in item["targets"]), "接入版本身份无效")
        if version != "latest" and release_version != version:
            continue
        if version == "latest" and not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", release_version):
            continue
        if release_version not in seen:
            seen.add(release_version)
            candidates.append(item)
    candidates.sort(key=lambda item: tuple(int(part) for part in item["version"].split("-")[0].split("+")[0].split(".")), reverse=True)
    ensure(candidates, f"面板未导入本机 {target} 可用的签名 Agent 版本，请先导入 Release")
    return candidates


def select_artifact(metadata, version, actual, requested="auto"):
    if not actual.startswith("linux-") and not re.fullmatch(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)", version):
        raise IncompatibleRelease("原生平台服务安装仅支持稳定版本，请选择数字三段版本")
    ensure(metadata["tag"] == "agent-v" + version, "签名发布版本与所选版本不匹配")
    if not metadata["protocol_min"] <= PROTOCOL_VERSION <= metadata["protocol_max"]:
        raise IncompatibleRelease("签名 Agent 发布不支持此接入入口的协议版本")
    for target in compatible_targets(actual, requested):
        matches = [item for item in metadata["artifacts"] if
                   (item["name"], item["version"], item["arch"]) == ("agent", version, target)]
        if matches:
            ensure(len(matches) == 1, "签名发布重复了本机 Agent 身份")
            item = matches[0]
            ensure(item["format"] == "raw" and item["binary_name"] == "sinan-agent"
                   and item["archive_size"] == item["binary_size"], "签名 Agent 格式不匹配")
            return item
    raise IncompatibleRelease(f"签名发布 {version} 没有本机 {actual} 可运行的 Agent")


def download_agent(item, destination, mirror="", release_dir=None):
    """Fetch only the signed raw asset from its official GitHub tag, never the panel."""
    expected_size = item["archive_size"]
    name = item["asset_name"]
    ensure(type(expected_size) is int and 0 < expected_size <= 256 * 1024 * 1024
           and item["format"] == "raw" and item["binary_size"] == expected_size,
           "已签 Agent 大小或格式无效")
    ensure(VERSION.fullmatch(item["version"]) and Path(name).name == name
           and name not in (".", ".."), "已签 Agent 身份无效")
    mirror = validate_mirror(mirror)
    destination = Path(destination)
    try:
        if release_dir:
            payload = read_regular(Path(release_dir) / name, expected_size)
            ensure(len(payload) == expected_size and digest(payload) == item["binary_sha256"],
                   "离线 Agent 不符合已签大小或摘要，拒绝执行")
            with destination.open("xb") as output:
                output.write(payload)
            return
        url = f"https://github.com/{REPOSITORY}/releases/download/agent-v{item['version']}/{urllib.parse.quote(name, safe='')}"
        if mirror:
            url = mirror + "/" + url
        validate_github_url(url, mirror)
        deadline = time.monotonic() + DOWNLOAD_BUDGET_SECONDS
        with github_opener(mirror).open(url, timeout=DOWNLOAD_SOCKET_TIMEOUT) as response:
            validate_github_url(response.url, mirror)
            ensure(response.status == 200, "GitHub Agent 响应无效")
            declared = response.headers.get("Content-Length")
            ensure(declared is None or int(declared) == expected_size, "GitHub Agent 声明长度不匹配")
            total, checksum = 0, hashlib.sha256()
            with destination.open("xb") as output:
                while True:
                    data = bounded_read(response, min(65536, expected_size + 1 - total), deadline)
                    if not data:
                        break
                    total += len(data)
                    ensure(total <= expected_size, "Agent 超出已签大小")
                    output.write(data)
                    checksum.update(data)
            ensure(total == expected_size and checksum.hexdigest() == item["binary_sha256"],
                   "Agent 不符合已签大小或摘要，拒绝执行")
    except BaseException:
        destination.unlink(missing_ok=True)
        raise


def checked_agent(agent, arguments):
    result = subprocess.run([str(agent)] + arguments, check=False)
    ensure(result.returncode == 0, "Agent 验证、接入或服务安装失败；已有版本未通过切换验收")


def native_paths(actual):
    configuration = Path("/private/etc/sinan/agent.toml" if actual == "macos-arm64" else "/etc/sinan/agent.toml")
    base = Path("/private/var" if actual == "macos-arm64" else "/var")
    return configuration, Path("/usr/local/bin/sinan-agent"), base, Path("/opt/sinan/core")


def normalized_origin(value):
    validate_panel_origin(value)
    parsed = urllib.parse.urlsplit(value)
    hostname = parsed.hostname.lower()
    try:
        hostname = ipaddress.ip_address(hostname).compressed
    except ValueError:
        hostname = hostname.encode("idna").decode("ascii")
    return parsed.scheme.lower(), hostname, parsed.port or (443 if parsed.scheme == "https" else 80)


def validate_partial_identity(identity, panel):
    if not identity.exists() and not identity.is_symlink():
        return
    require_protected_file(identity)
    ensure(identity.is_dir() and not identity.is_symlink(), "首次接入身份目录必须是普通受控目录")
    names = {path.name for path in identity.iterdir()}
    if not names:
        return
    ensure("panel_origin" in names and names <= {"device.key", "panel_origin", "server_id"}
           and ("server_id" not in names or "device.key" in names),
           "已有身份不是可恢复的首次接入状态，请先核对已有安装")
    key = identity / "device.key"
    if "device.key" in names:
        require_protected_file(key)
        ensure(key.stat().st_mode & 0o077 == 0 and len(read_regular(key, 32)) == 32,
               "首次接入的设备密钥必须是私有 32 字节文件")
    origin = identity / "panel_origin"
    require_protected_file(origin)
    recorded = read_regular(origin, 8192).decode("utf-8").strip()
    ensure(normalized_origin(recorded) == normalized_origin(panel),
           "首次接入身份属于另一面板，请使用原面板重新获取令牌")
    server = identity / "server_id"
    if server.exists() or server.is_symlink():
        require_protected_file(server)
        value = read_regular(server, 32).decode("ascii").strip()
        ensure(re.fullmatch(r"[1-9][0-9]{0,18}", value) and int(value) <= 9223372036854775807,
               "首次接入的服务器身份无效")


def install_native(bundle, panel, token, item, actual, mirror="", release_dir=None):
    import tomllib

    agent = bundle / "sinan-agent"
    download_agent(item, agent, validate_mirror(mirror, panel), release_dir)
    agent.chmod(0o755)
    checked_agent(agent, ["verify-installed", "--binary", str(agent), "--name", "agent", "--format", "raw"])
    configuration, command, base, agent_root = native_paths(actual)
    if command.exists() or command.is_symlink():
        ensure(command.is_symlink(), "已有 /usr/local/bin/sinan-agent 普通文件，请先核对安装来源")
    if command.parent.exists():
        require_protected_file(command.parent)
    previous_configuration = None
    if configuration.exists() or configuration.is_symlink():
        require_protected_file(configuration)
        checked_agent(agent, ["--config", str(configuration), "verify-cache"])
        saved = tomllib.loads(read_regular(configuration, 1048576).decode("utf-8"))
        previous_configuration = configuration.read_bytes()
        if "agent_root" in saved:
            ensure(isinstance(saved["agent_root"], str) and Path(saved["agent_root"]).is_absolute(), "已有 Agent 安装目录无效")
            agent_root = Path(saved["agent_root"])
    else:
        identity = configuration.parent / "identity"
        roots = [base / ("lib/sinan/core/state.db" + suffix) for suffix in ("", "-wal", "-shm")]
        roots.append(agent_root / "current")
        roots.extend(Path("/opt/sinan/plugins").glob("*/current"))
        ensure(not any(path.exists() or path.is_symlink() for path in roots),
               "已有 Agent 状态但缺少配置，无法安全预检；请先修复已有安装")
        validate_partial_identity(identity, panel)
    previous_agent = agent_root / "current/sinan-agent"
    if previous_agent.exists():
        previous_agent = previous_agent.resolve(strict=True)
        checked_agent(agent, ["verify-installed", "--binary", str(previous_agent), "--name", "agent", "--format", "raw"])
    try:
        checked_agent(agent, ["--config", str(configuration), "enroll", "--panel", panel, "--token", token])
        checked_agent(agent, ["--config", str(configuration), "install-service"])
    except (ValueError, OSError):
        if previous_configuration is not None:
            with tempfile.NamedTemporaryFile(prefix=".sinan-config-rollback-", dir=configuration.parent, delete=False) as output:
                restoration = Path(output.name)
                output.write(previous_configuration)
                output.flush()
                os.fsync(output.fileno())
            restoration.chmod(0o600)
            restoration.replace(configuration)
            # The native CLI restores its previous current link; reload its restored configuration.
            if previous_agent.exists():
                checked_agent(agent, ["verify-installed", "--binary", str(previous_agent), "--name", "agent", "--format", "raw"])
                checked_agent(previous_agent, ["--config", str(configuration), "install-service"])
        raise
    command.parent.mkdir(parents=True, exist_ok=True)
    temporary = command.with_name(".sinan-agent-" + str(os.getpid()))
    temporary.symlink_to(agent_root / "current/sinan-agent")
    temporary.replace(command)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", help="Legacy exact release tag")
    parser.add_argument("--version", help="Exact Agent version or latest; defaults to latest")
    parser.add_argument("--target", default="auto", help="Expected server platform or auto")
    parser.add_argument("--panel", required=True)
    parser.add_argument("--token")
    parser.add_argument("--mirror", default="", help="HTTPS prefix for GitHub downloads")
    parser.add_argument("--trusted-keys", default="/etc/sinan/trust/public-keys.json",
                        help="Operator-provisioned root-owned JSON key set, independent of panel")
    parser.add_argument("--minisign", default="minisign")
    parser.add_argument("--trusted-installer", help="Independent root-protected Linux installer; defaults to bundled trusted-install.sh")
    parser.add_argument("--trusted-agent", help="Previously trusted signed Agent for offline proof verification")
    parser.add_argument("--release-dir", help="Pre-downloaded signed release, including Agent binary; offline installation")
    args = parser.parse_args()
    ensure(os.getuid() == 0, "bootstrap requires root")
    if args.tag:
        ensure(args.tag.startswith("agent-v") and VERSION.fullmatch(args.tag[7:]), "invalid tag")
        ensure(args.version is None or args.version == args.tag[7:], "--tag 与 --version 不一致")
    version = args.tag[7:] if args.tag else args.version or "latest"
    ensure(version == "latest" or VERSION.fullmatch(version), "无效 Agent 版本")
    validate_panel_origin(args.panel)
    actual = host_target()
    compatible_targets(actual, args.target)
    mirror = validate_mirror(args.mirror, args.panel)
    token = args.token or os.environ.pop("SINAN_ENROLLMENT_TOKEN", None)
    ensure(token, "provide one-time token through SINAN_ENROLLMENT_TOKEN")
    roots = None if args.trusted_agent else load_roots(args.trusted_keys, require_protected=True)
    if args.release_dir:
        if version == "latest":
            unverified = json.loads(read_regular(Path(args.release_dir) / "release.json", 32768))
            tag = unverified.get("tag", "")
            ensure(isinstance(tag, str) and tag.startswith("agent-v") and VERSION.fullmatch(tag[7:]), "离线 Release 标签无效")
            version = tag[7:]
        candidates = [{"version": version, "tag": "agent-v" + version}]
    elif args.tag:
        candidates = [{"version": version, "tag": args.tag}]
    else:
        candidates = catalog(args.panel, token, actual if args.target == "auto" else args.target, version)
    os.umask(0o077)
    temporary_root = Path("/opt/sinan")
    temporary_root.mkdir(mode=0o755, parents=True, exist_ok=True)
    require_protected_file(temporary_root)
    with tempfile.TemporaryDirectory(prefix="sinan-bootstrap-", dir=temporary_root) as temporary:
        bundle = None
        item = None
        for candidate in candidates:
            release_version, tag = candidate["version"], candidate["tag"]
            selected = Path(temporary) / release_version
            selected.mkdir()
            base = f"https://github.com/{REPOSITORY}/releases/download/{tag}"
            for name, limit in PROOF_FILES:
                if args.release_dir:
                    (selected / name).write_bytes(read_regular(Path(args.release_dir) / name, limit))
                else:
                    download(base, name, selected / name, limit, mirror)
            if args.trusted_agent:
                trusted_agent = Path(args.trusted_agent).resolve(strict=True)
                require_protected_file(trusted_agent)
                ensure(trusted_agent.name == "sinan-agent", "trusted verifier must be the installed Agent")
                for command in ([str(trusted_agent), "verify-installed", "--binary", str(trusted_agent),
                                 "--name", "agent", "--format", "raw"],
                                [str(trusted_agent), "verify-release", "--proof-dir", str(selected)]):
                    result = subprocess.run(command, capture_output=True, check=False)
                    ensure(result.returncode == 0, "previous Agent refused release verification")
                metadata, _ = validate_manifest(selected, tag, protocol_version=None)
            else:
                metadata, _ = verify_manifest(selected, roots, args.minisign, tag, protocol_version=None)
            try:
                item = select_artifact(metadata, release_version, actual, args.target)
            except IncompatibleRelease:
                if version != "latest":
                    raise
                continue
            bundle, version = selected, release_version
            break
        ensure(bundle is not None and item is not None, f"没有通过签名和本机 {actual} 兼容检查的 Agent 版本")
        print(f"已验证 Agent {version}，安装平台 {item['arch']}（本机 {actual}）", flush=True)
        if not actual.startswith("linux-"):
            install_native(bundle, args.panel, token, item, actual, mirror, args.release_dir)
            return

        legacy_checkpoint_preflight(version)

        installer = Path(args.trusted_installer) if args.trusted_installer else Path(__file__).with_name("trusted-install.sh")
        require_protected_file(installer)
        ensure(installer.is_file(), "独立可信 Linux 安装器缺失，请使用官方自包含 bootstrap.sh")
        # The fixed, independently verified bootstrap supplies this executor.
        # Legacy signed install.sh is proof material only and is never executed.
        require_preloaded_installer(installer)
        download_agent(item, bundle / item["asset_name"], mirror, args.release_dir)
        token_file = bundle / ".enrollment-token"
        token_file.write_text(token)
        command = ["/bin/sh", str(installer), "--bundle", str(bundle),
                   "--panel", args.panel, "--version", version, "--token-file", str(token_file)]
        if item["arch"] not in ("amd64", "arm64"):
            command.extend(["--target", item["arch"]])
        result = subprocess.run(command, check=False)
        raise SystemExit(result.returncode)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise SystemExit(f"Bootstrap refused: {error}") from error
SINAN_BOOTSTRAP_7415F3BDC30846B6CEC2230C27EC5FF57840ABA1FA0BC3BCA2E47C30DA90269B

cat > "$STAGING/legacy_agent_checkpoint.py" <<'SINAN_BOOTSTRAP_3787069DD3526732BC6A95C780003451986974D878DB9DFA33BDE240E46770DB'
#!/usr/bin/env python3
"""Read-only recovery gate for independently signed Agent versions before 0.3.1."""

import errno
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import time
import urllib.parse
import uuid

SEGMENT = re.compile(r"[0-9A-Za-z][0-9A-Za-z.+_-]{0,127}\Z")
LEGACY_NODEQUALITY_VERSION = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2"


def ensure(condition, message):
    if not condition:
        raise ValueError(message)


def protected(path):
    for part in (path,) + tuple(path.parents):
        properties = part.lstat()
        ensure(not part.is_symlink() and properties.st_uid == 0 and properties.st_mode & 0o022 == 0,
               "旧 Agent 状态路径必须由 root 保护，安装未切换")


def regular(path, limit):
    protected(path)
    ensure(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= limit,
           "旧 Agent 状态必须是有界普通文件，安装未切换")
    return path.read_bytes()


def quiescent(saved, allow_missing=False):
    """An old process must not create Preparing after the read-only preflight."""
    environment = os.environ.copy()
    environment["LC_ALL"] = "C"
    try:
        if Path("/run/systemd/system").is_dir():
            result = subprocess.run(["systemctl", "show", "sinan-agent.service", "--property=LoadState",
                                     "--property=ActiveState", "--property=SubState", "--property=MainPID"],
                                    capture_output=True, env=environment, check=False, timeout=3)
            ensure(result.returncode == 0 and len(result.stdout) <= 4096,
                   "无法确认旧 Agent 服务停止，请先执行受控维护，安装未切换")
            properties = {}
            for line in result.stdout.decode("ascii").splitlines():
                key, separator, value = line.partition("=")
                ensure(separator and key not in properties, "旧 Agent 服务状态无效，安装未切换")
                properties[key] = value
            ensure(set(properties) == {"LoadState", "ActiveState", "SubState", "MainPID"}
                   and properties["LoadState"] in (("loaded", "not-found") if allow_missing else ("loaded",))
                   and properties["ActiveState"] == "inactive" and properties["SubState"] == "dead"
                   and properties["MainPID"] == "0", "旧 Agent 仍可能运行，请先执行受控维护，安装未切换")
        elif Path("/run/openrc/softlevel").is_file():
            exists = subprocess.run(["rc-service", "--exists", "sinan-agent"], capture_output=True,
                                    env=environment, check=False, timeout=3)
            status = subprocess.run(["rc-service", "sinan-agent", "status"], capture_output=True,
                                    env=environment, check=False, timeout=3)
            ensure((exists.returncode == 0 and status.returncode == 3)
                   or (allow_missing and exists.returncode == 1 and status.returncode == 1),
                   "无法确认旧 Agent 已停止，请先执行受控维护，安装未切换")
        else:
            raise ValueError("无法确认旧 Agent 服务状态，请先执行受控维护，安装未切换")
    except (OSError, subprocess.TimeoutExpired, UnicodeError):
        raise ValueError("无法确认旧 Agent 服务停止，请先执行受控维护，安装未切换") from None
    endpoint = saved.get("status_socket", "/run/sinan/agent.sock")
    ensure(isinstance(endpoint, str) and Path(endpoint).is_absolute()
           and ".." not in Path(endpoint).parts and not any(ord(character) < 32 for character in endpoint),
           "旧 Agent 状态端点无效，安装未切换")
    connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    connection.settimeout(0.5)
    try:
        connection.connect(endpoint)
    except OSError as error:
        ensure(error.errno in (errno.ENOENT, errno.ECONNREFUSED),
               "无法确认旧 Agent 状态端点停止，安装未切换")
    else:
        raise ValueError("旧 Agent 状态端点仍在运行，请先执行受控维护，安装未切换")
    finally:
        connection.close()


def unique_object(pairs):
    value = {}
    for key, entry in pairs:
        ensure(key not in value, "旧诊断状态字段重复，安装未切换")
        value[key] = entry
    return value


def options_valid(value):
    return (isinstance(value, dict) and len(value) <= 32
            and all(isinstance(key, str) and len(key) <= 128 and isinstance(entry, str)
                    and len(entry) <= 1024 for key, entry in value.items()))


def optional_fields_valid(value):
    return ((value.get("expires_at") is None or type(value["expires_at"]) is int
             and -(2 ** 63) <= value["expires_at"] < 2 ** 63)
            and all(value.get(key) is None or isinstance(value[key], str) and len(value[key]) <= 4096
                    for key in ("start_error", "protection_stop_reason")))


def checkpoint_safe(encoded):
    try:
        checkpoint = json.loads(encoded, object_pairs_hook=unique_object,
                                parse_constant=lambda _value: (_ for _ in ()).throw(ValueError()))
    except (ValueError, RecursionError):
        raise ValueError("旧诊断状态无法安全解析，安装未切换") from None
    if checkpoint is None:
        return
    ensure(isinstance(checkpoint, dict) and len(checkpoint) == 1
           and next(iter(checkpoint)) in ("Preparing", "Started"), "旧诊断检查点未知，安装未切换")
    phase = next(iter(checkpoint))
    value = checkpoint[phase]
    ensure(isinstance(value, dict), "旧诊断检查点损坏，安装未切换")
    if phase == "Started":
        ensure({"spec", "service", "plugin", "started_at"} <= set(value)
               and set(value) <= {"spec", "service", "plugin", "started_at", "start_error",
                                  "expires_at", "protection_stop_reason", "environment"}
               and isinstance(value["spec"], dict) and isinstance(value["service"], dict)
               and value["plugin"] == "nodequality" and type(value["started_at"]) is int
               and 0 <= value["started_at"] < 2 ** 64 and optional_fields_valid(value)
               and (value.get("environment") is None or isinstance(value["environment"], dict)),
               "旧已启动检查点损坏，安装未切换")
        spec, service = value["spec"], value["service"]
        ensure(set(spec) == {"id", "version", "binary_path", "job_dir", "timeout_secs", "options"}
               and {"unit", "program", "args", "working_directory", "timeout_secs"} <= set(service)
               and set(service) <= {"unit", "program", "args", "working_directory", "timeout_secs",
                                    "memory_max", "tasks_max", "cpu_weight", "io_weight", "oom_score_adjust"}
               and isinstance(spec.get("version"), str) and SEGMENT.fullmatch(spec["version"])
               and isinstance(spec.get("id"), str) and len(spec["id"]) == 36
               and isinstance(spec.get("binary_path"), str) and Path(spec["binary_path"]).is_absolute()
               and isinstance(spec.get("job_dir"), str) and Path(spec["job_dir"]).is_absolute()
               and type(spec.get("timeout_secs")) is int and 1 <= spec["timeout_secs"] <= 3600
               and type(service.get("timeout_secs")) is int and service["timeout_secs"] == spec["timeout_secs"]
               and options_valid(spec.get("options")) and isinstance(service.get("args"), list)
               and len(service["args"]) <= 64
               and all(isinstance(argument, str) and len(argument) <= 4096 for argument in service["args"])
               and service.get("working_directory") == spec["job_dir"]
               and service.get("program") == spec["binary_path"]
               and service.get("unit") == "sinan-diagnostic-" + spec["id"] + ".service",
               "旧已启动检查点版本或执行身份无效，安装未切换")
        ensure(all(type(service[key]) is int for key in
                   {"memory_max", "tasks_max", "cpu_weight", "io_weight", "oom_score_adjust"} & set(service)),
               "旧已启动检查点资源形状无效，安装未切换")
        ensure(spec["version"] == LEGACY_NODEQUALITY_VERSION
               and set(spec["options"]) <= {"ip_version", "network_mode", "upload_report"}
               and spec["options"].get("ip_version", "both") in ("both", "ipv4", "ipv6")
               and spec["options"].get("network_mode", "low") in ("low", "normal")
               and spec["options"].get("upload_report", "false") in ("true", "false"),
               "旧 Agent 无法按原版本回收此已启动任务；请保留状态并使用兼容的新签名 Agent，安装未切换")
        try:
            uuid.UUID(spec["id"])
        except (ValueError, TypeError, AttributeError):
            raise ValueError("旧已启动检查点身份无效，安装未切换") from None
        # Existing compiled-root verify-cache validates the exact saved executable proof;
        # the old Agent only observes Started instead of executing it again.
        return
    ensure({"id", "version", "artifact", "timeout_secs"} <= set(value)
           and set(value) <= {"id", "version", "artifact", "timeout_secs", "plugin", "options",
                              "expires_at", "resource_budget"}
           and isinstance(value["version"], str) and SEGMENT.fullmatch(value["version"])
           and type(value["timeout_secs"]) is int and 1 <= value["timeout_secs"] <= 3600
           and isinstance(value["artifact"], dict) and optional_fields_valid(value),
           "旧排队检查点损坏，安装未切换")
    try:
        ensure(isinstance(value["id"], str) and len(value["id"]) == 36, "invalid checkpoint UUID")
        uuid.UUID(value["id"])
    except (ValueError, TypeError, AttributeError):
        raise ValueError("旧排队检查点身份无效，安装未切换") from None
    artifact = value["artifact"]
    proof = artifact.get("proof")
    ensure(set(artifact) == {"url", "sha256", "proof"} and isinstance(artifact["url"], str)
           and 0 < len(artifact["url"]) <= 8192 and isinstance(artifact["sha256"], str)
           and re.fullmatch(r"[0-9a-f]{64}", artifact["sha256"])
           and isinstance(proof, dict) and set(proof) == {"metadata_json", "checksums", "signature"}
           and all(isinstance(proof[key], str) and 0 < len(proof[key]) <= limit
                   for key, limit in (("metadata_json", 32768), ("checksums", 8192), ("signature", 16384))),
           "旧排队检查点签名证明形状无效，安装未切换")
    options = value.get("options", {})
    budget = value.get("resource_budget")
    ensure(budget is None or isinstance(budget, dict)
           and set(budget) == {"memory_max", "tasks_max", "cpu_weight", "io_weight", "oom_score_adjust"}
           and all(type(entry) is int for entry in budget.values()),
           "旧排队检查点资源形状无效，安装未切换")
    ensure(options_valid(options) and value.get("plugin", "nodequality") == "nodequality",
           "旧排队检查点插件或参数未知，安装未切换")
    mode = options.get("mode", "full")
    ensure(mode in ("full", "daily"), "旧排队检查点模式未知，安装未切换")
    ensure(mode != "full", "旧 Agent 存在尚未启动的完整验机任务；恢复门禁拒绝，原状态保留，安装未切换")


def _preflight(version, configuration):
    ensure(isinstance(version, str) and re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?", version),
           "旧 Agent 版本无法安全确认")
    if tuple(int(part) for part in version.split("-")[0].split("+")[0].split(".")) > (0, 3, 0):
        return
    configuration = Path(configuration)
    database, saved = Path("/var/lib/sinan/core/state.db"), {}
    if configuration.exists() or configuration.is_symlink():
        try:
            import tomllib
        except ImportError:
            raise ValueError("旧 Agent 恢复预检需要 Python 3.11 或更新版本，安装未切换") from None
        try:
            saved = tomllib.loads(regular(configuration, 1048576).decode("utf-8"))
        except (ValueError, UnicodeError):
            raise ValueError("旧 Agent 配置无法安全解析，安装未切换") from None
        state_path = saved.get("state_db", str(database))
        ensure(isinstance(state_path, str) and Path(state_path).is_absolute()
               and not any(ord(character) < 32 for character in state_path)
               and ".." not in Path(state_path).parts, "旧 Agent 状态位置无法安全确认，安装未切换")
        database = Path(state_path)
        quiescent(saved, allow_missing=not database.exists())
    if not database.exists() and not database.is_symlink():
        return
    if not (configuration.exists() or configuration.is_symlink()):
        raise ValueError("旧 Agent 状态存在但配置缺失，安装未切换")
    protected(database)
    ensure(database.is_file(), "旧 Agent 状态必须是普通文件，安装未切换")
    wal, shm = (Path(str(database) + suffix) for suffix in ("-wal", "-shm"))
    for sidecar in (wal, shm):
        if sidecar.exists() or sidecar.is_symlink():
            protected(sidecar)
            ensure(sidecar.is_file(), "旧 Agent 状态附属文件无效，安装未切换")
    ensure(not wal.exists() or wal.stat().st_size == 0 or shm.exists(),
           "旧 Agent WAL 缺少共享内存，拒绝创建恢复文件，安装未切换")
    try:
        import sqlite3
    except ImportError:
        raise ValueError("旧 Agent 状态预检需要 Python 标准库 SQLite 支持，安装未切换") from None
    status = lambda: [(path.exists(), path.stat().st_ino, path.stat().st_size, path.stat().st_mtime_ns)
                      if path.exists() else (False,) for path in (database, wal, shm)]
    before, deadline = status(), time.monotonic() + 1
    connection = None
    try:
        uri = "file:" + urllib.parse.quote(str(database), safe="/") + "?mode=ro"
        connection = sqlite3.connect(uri, uri=True, timeout=1)
        connection.set_progress_handler(lambda: int(time.monotonic() >= deadline), 1000)
        rows = connection.execute(
            "SELECT substr(value,1,1048577),typeof(value) FROM kv WHERE key='diagnostics:active' LIMIT 2").fetchall()
    except sqlite3.Error:
        raise ValueError("旧诊断状态无法只读确认，安装未切换") from None
    finally:
        if connection is not None:
            connection.close()
    ensure(status() == before, "旧诊断状态在预检期间变化，安装未切换")
    ensure(len(rows) <= 1, "旧诊断状态重复，安装未切换")
    if rows:
        encoded, kind = rows[0]
        ensure(kind == "text" and isinstance(encoded, str) and len(encoded.encode("utf-8")) <= 1048576,
               "旧诊断状态不是有界 JSON，安装未切换")
        checkpoint_safe(encoded)


def preflight(version, configuration=Path("/etc/sinan/agent.toml")):
    try:
        _preflight(version, configuration)
    except OSError:
        raise ValueError("旧状态路径无法安全读取，安装未切换") from None


if __name__ == "__main__":
    try:
        ensure(len(sys.argv) == 2, "旧 Agent 预检参数无效")
        preflight(sys.argv[1])
    except ValueError as error:
        raise SystemExit(f"Legacy Agent refused: {error}") from None
    except OSError:
        raise SystemExit("Legacy Agent refused: 旧状态路径无法安全读取，安装未切换") from None
SINAN_BOOTSTRAP_3787069DD3526732BC6A95C780003451986974D878DB9DFA33BDE240E46770DB

cat > "$STAGING/release.py" <<'SINAN_BOOTSTRAP_A9D893044F3B931A371AF1C8209993857DFB8FC3920504AE95D03374AB5A7DEF'
#!/usr/bin/env python3
"""Build canonical release manifests and verify complete offline-signed bundles."""

import argparse
import base64
import hashlib
import gzip
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile


REPOSITORY = "theLucius7/sinan"
TEST_ONLY_PUBLIC_KEY = "RWS3NbDikg3VqWRlxJMUyaB1dTvErk0ptJ695xQ50Kyb+MmtynMhN/lq"
TEST_ONLY_ROTATION_PUBLIC_KEY = "RWRURVNUUk9UMjMuvo0ny3Mjs6QBwcE7XdZLzMDhDs2hwrXRGgN3moXl"
SOURCE_ROOT = Path(__file__).resolve().parents[1]
TEST_PUBLIC_KEY_DIRS = (SOURCE_ROOT / "fixtures", SOURCE_ROOT / "crates/protocol/tests/fixtures")
NODEQUALITY_VERSION = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r14"
SEGMENT = re.compile(r"[0-9A-Za-z][0-9A-Za-z.+_-]{0,127}\Z")
VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?\Z")
MAX_BINARY = 256 * 1024 * 1024


def ensure(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_regular(path, limit=MAX_BINARY):
    ensure(path.is_file() and not path.is_symlink(), "asset must be an ordinary file")
    ensure(0 < path.stat().st_size <= limit, "asset size outside permitted range")
    return path.read_bytes()


def canonical_path(entry):
    ensure(entry["name"] in ("agent", "sing-box", "nodequality", "tcpquality"), "unsupported module")
    ensure(SEGMENT.fullmatch(entry["version"]), "invalid version segment")
    ensure(entry["arch"] in ("amd64", "arm64", "linux-gnu-amd64", "linux-gnu-arm64", "linux-musl-amd64", "linux-musl-arm64", "macos-arm64", "freebsd-amd64", "freebsd-arm64", "windows-amd64", "windows-arm64"), "unsupported architecture")
    return "/".join(entry[k] for k in ("name", "version", "arch"))


def asset_name(entry):
    name, version, arch = (entry[k] for k in ("name", "version", "arch"))
    ensure(entry["format"] in ("raw", "tar.gz"), "unsupported format")
    if arch not in ("amd64", "arm64"):
        return f"{name}-{version}-{arch}" + (".tar.gz" if entry["format"] == "tar.gz" else "")
    return (f"{name}-{version}-linux-musl-{arch}" if entry["format"] == "raw"
            else f"{name}-{version}-linux-{arch}.tar.gz")


def validate_auxiliary_files(value, binary_name, archive_format):
    ensure(isinstance(value, dict) and len(value) <= 7, "invalid auxiliary file list")
    ensure(not value or archive_format == "tar.gz", "raw artifacts cannot contain auxiliary files")
    for name, entry in value.items():
        ensure(isinstance(name, str) and re.fullmatch(r"[0-9A-Za-z._-]{1,128}", name)
               and name not in (".", "..", binary_name, "release.json", "SHA256SUMS",
                                "SHA256SUMS.minisig", ".artifact.json") and not name.startswith("-"),
               "invalid auxiliary file name")
        ensure(isinstance(entry, dict) and set(entry) == {"sha256", "size"},
               "invalid auxiliary file metadata")
        ensure(type(entry["size"]) is int and 0 < entry["size"] <= MAX_BINARY,
               "invalid auxiliary file size")
        ensure(isinstance(entry["sha256"], str) and re.fullmatch(r"[0-9a-f]{64}", entry["sha256"]),
               "invalid auxiliary file digest")
    return value


def binary_bytes(data, archive_format, binary_name, auxiliary_files=None):
    auxiliary = validate_auxiliary_files(auxiliary_files if auxiliary_files is not None else {},
                                         binary_name, archive_format)
    if archive_format == "raw":
        return data
    ensure(archive_format == "tar.gz", "unsupported format")
    expected = {binary_name} | set(auxiliary)
    seen, binary, consumed = set(), None, 0
    with gzip.GzipFile(fileobj=io.BytesIO(data), mode="rb") as compressed:
        def read(size):
            nonlocal consumed
            content = compressed.read(min(size, MAX_BINARY - consumed + 1))
            consumed += len(content)
            ensure(consumed <= MAX_BINARY, "archive exceeds unpacked size limit")
            return content

        while True:
            header = read(tarfile.BLOCKSIZE)
            ensure(len(header) in (0, tarfile.BLOCKSIZE), "truncated archive header")
            if not header or not any(header):
                break
            try:
                member = tarfile.TarInfo.frombuf(header, encoding="utf-8", errors="strict")
            except tarfile.HeaderError as error:
                raise ValueError("invalid archive header") from error
            ensure(member.name in expected and member.name not in seen
                   and member.type in (tarfile.REGTYPE, tarfile.AREGTYPE), "unsafe archive member")
            seen.add(member.name)
            ensure(0 < member.size <= MAX_BINARY, "invalid archive member size")
            if member.name != binary_name:
                ensure(member.size == auxiliary[member.name]["size"], "auxiliary file size mismatch")
            content = read(member.size)
            ensure(len(content) == member.size, "archive file length mismatch")
            if member.name == binary_name:
                binary = content
            else:
                ensure(digest(content) == auxiliary[member.name]["sha256"], "auxiliary file digest mismatch")
            padding = (-member.size) % tarfile.BLOCKSIZE
            ensure(len(read(padding)) == padding, "truncated archive padding")
        ensure(seen == expected, "archive does not contain the exact signed file set")
        while True:
            tail = read(8192)
            if not tail:
                break
            ensure(not any(tail), "archive contains trailing data")
    return binary


def assemble(args):
    ensure(VERSION.fullmatch(args.agent_version), "invalid Agent version")
    ensure(args.tag == "agent-v" + args.agent_version, "tag differs from Agent version")
    source, output = Path(args.source), Path(args.output)
    ensure(not output.exists(), "release output already exists")
    output.mkdir(parents=True)
    paths, artifacts = {}, []
    architectures = getattr(args, "arch", None) or ("amd64", "arm64")
    ensure(len(set(architectures)) == len(architectures)
           and set(architectures) <= {"amd64", "arm64"}, "invalid or duplicate architecture")
    modules = [
        ("agent", args.agent_version, "raw", "sinan-agent"),
        ("sing-box", args.runtime_version, "tar.gz", "sing-box"),
        ("nodequality", getattr(args, "nodequality_version", NODEQUALITY_VERSION), "tar.gz", "nodequality"),
    ]
    if getattr(args, "tcp_probe_version", None) is not None:
        modules.append(("tcpquality", args.tcp_probe_version, "tar.gz", "sinan-tcp-probe"))
    for name, version, archive_format, binary_name in modules:
        for arch in architectures:
            entry = {"name": name, "version": version, "arch": arch,
                     "format": archive_format, "binary_name": binary_name}
            path = canonical_path(entry)
            data = read_regular(source / path)
            auxiliary = {}
            if name == "tcpquality":
                from tcp_probe_artifact import archive_files, validate_files
                files = archive_files(data)
                validate_files(files, version, arch)
                auxiliary = {name: {"sha256": digest(content), "size": len(content)}
                             for name, content in files.items() if name != binary_name}
                entry["auxiliary_files"] = auxiliary
            binary = binary_bytes(data, archive_format, binary_name, auxiliary)
            entry.update(archive_size=len(data), binary_sha256=digest(binary),
                         binary_size=len(binary), asset_name=asset_name(entry))
            (output / entry["asset_name"]).write_bytes(data)
            paths[path] = digest(data)
            artifacts.append(entry)
    artifacts.sort(key=canonical_path)
    metadata = {"schema": 1, "source_repo": REPOSITORY, "tag": args.tag,
                "protocol_min": 1, "protocol_max": 1, "artifacts": artifacts}
    encoded = (json.dumps(metadata, sort_keys=True, separators=(",", ":")) + "\n").encode()
    ensure(len(encoded) <= 32768, "metadata too large")
    (output / "release.json").write_bytes(encoded)
    installer = read_regular(Path(args.installer), 256 * 1024)
    (output / "install.sh").write_bytes(installer)
    paths.update({"release.json": digest(encoded), "install.sh": digest(installer)})
    checksums = "".join(f"{paths[path]}  {path}\n" for path in sorted(paths))
    ensure(len(checksums.encode()) <= 8192, "checksums too large")
    (output / "SHA256SUMS").write_bytes(checksums.encode("utf-8"))


def installer_source(template, agent_unit, runtime_unit, source_root=SOURCE_ROOT):
    """Render audited static Linux installation logic for release or trusted bootstrap."""
    text = read_regular(Path(template), 262144).decode("utf-8")
    if "@@LEGACY_CHECKPOINT_PREFLIGHT@@" in text:
        ensure(text.count("@@LEGACY_CHECKPOINT_PREFLIGHT@@") == 2, "missing or duplicate legacy preflight marker")
        guard = read_regular(Path(source_root) / "tools/legacy_agent_checkpoint.py", 65536).decode("utf-8")
        text = text.replace("@@LEGACY_CHECKPOINT_PREFLIGHT@@", guard.rstrip())
    for marker, filename in (("@@AGENT_UNIT@@", agent_unit),
                             ("@@RUNTIME_UNIT@@", runtime_unit)):
        ensure(text.count(marker) == 1, "missing or duplicate installer unit marker")
        text = text.replace(marker, read_regular(Path(filename), 65536).decode("utf-8").rstrip())
    for marker, filename in (("@@AGENT_OPENRC@@", source_root / "deploy/sinan-agent.openrc"),
                             ("@@RUNTIME_OPENRC@@", source_root / "plugins/sing-box/sinan-singbox.openrc")):
        if marker in text:
            ensure(text.count(marker) == 1, "duplicate installer unit marker")
            text = text.replace(marker, read_regular(filename, 65536).decode("utf-8").rstrip())
    ensure("@@" not in text, "unexpanded installer marker")
    return text


def render_installer(args):
    text = installer_source(args.template, args.agent_unit, args.runtime_unit)
    output = Path(args.output)
    ensure(not output.exists(), "installer output exists")
    output.write_bytes(text.encode("utf-8"))


def public_record(key):
    ensure(isinstance(key, str) and len(key.encode()) <= 4096, "invalid public key")
    lines = key.splitlines()
    record = lines[-1] if len(lines) in (1, 2) else ""
    if len(lines) == 2:
        ensure(lines[0].startswith("untrusted comment: "), "invalid public key comment")
    binary = base64.b64decode(record, validate=True)
    ensure(len(binary) == 42 and binary[:2] == b"Ed", "invalid minisign public key")
    return record, binary[10:]


def test_public_material():
    # Include the known roots when trusted bootstrap tools are copied without tests.
    denied = {public_record(key)[1] for key in
              (TEST_ONLY_PUBLIC_KEY, TEST_ONLY_ROTATION_PUBLIC_KEY)}
    for directory in TEST_PUBLIC_KEY_DIRS:
        for path in directory.glob("TEST_ONLY*.pub"):
            denied.add(public_record(read_regular(path, 4096).decode("utf-8"))[1])
    return denied


def load_roots(path, publication=False, require_protected=False):
    path = Path(path)
    if require_protected:
        for part in (path,) + tuple(path.parents):
            stat = part.lstat()
            ensure(not part.is_symlink() and stat.st_uid == 0 and stat.st_mode & 0o022 == 0,
                   "bootstrap trust file and parents must be root-owned and protected")
    value = json.loads(read_regular(path, 32768))
    ensure(isinstance(value, list) and 0 < len(value) <= 8, "invalid trusted key set")
    denied = test_public_material() if publication else set()
    roots, public_material = [], set()
    for key in value:
        record, material = public_record(key)
        ensure(material not in public_material, "duplicate public key")
        if publication:
            ensure(material not in denied, "TEST_ONLY key cannot publish an official release")
        roots.append(record)
        public_material.add(material)
    return roots


def require_protected_file(path):
    path = Path(path)
    for part in (path,) + tuple(path.parents):
        stat = part.lstat()
        ensure(not part.is_symlink() and stat.st_uid == 0 and stat.st_mode & 0o022 == 0,
               "trusted file and parents must be root-owned and protected")


def verify_signature(bundle, roots, minisign):
    signature = read_regular(bundle / "SHA256SUMS.minisig", 16384).decode("utf-8")
    lines = signature.splitlines()
    ensure(len(lines) == 4 and lines[0].startswith("untrusted comment: ")
           and lines[2].startswith("trusted comment: "), "signature must contain all four minisign lines")
    ensure(base64.b64decode(lines[1], validate=True)[:2] == b"ED", "legacy signature prohibited")
    for key in roots:
        result = subprocess.run([minisign, "-V", "-H", "-q", "-m", str(bundle / "SHA256SUMS"),
                                 "-x", str(bundle / "SHA256SUMS.minisig"), "-P", key],
                                capture_output=True, check=False)
        if result.returncode == 0:
            return
    raise ValueError("no trusted key verifies the complete signature")


def verify_manifest(bundle, roots, minisign, expected_tag=None, protocol_version=1):
    verify_signature(Path(bundle), roots, minisign)
    return validate_manifest(bundle, expected_tag, protocol_version)


def validate_manifest(bundle, expected_tag=None, protocol_version=1):
    """Validate contents only after an independently successful signature verifier."""
    bundle = Path(bundle)
    ensure(not bundle.is_symlink(), "bundle must not be a symlink")
    checksums = read_regular(bundle / "SHA256SUMS", 8192).decode("utf-8")
    rows = {}
    for line in checksums.splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([0-9A-Za-z.+_/-]+)", line)
        ensure(match is not None, "noncanonical checksum entry")
        value, path = match.groups()
        ensure(path not in rows and not path.startswith("/") and ".." not in path.split("/"),
               "duplicate or unsafe checksum path")
        rows[path] = value
    canonical = "".join(f"{rows[path]}  {path}\n" for path in sorted(rows))
    ensure(checksums == canonical, "checksums must be sorted canonical LF text")
    metadata_bytes = read_regular(bundle / "release.json", 32768)
    ensure(rows.get("release.json") == digest(metadata_bytes), "metadata hash mismatch")
    metadata = json.loads(metadata_bytes)
    ensure(set(metadata) == {"schema", "source_repo", "tag", "protocol_min", "protocol_max", "artifacts"},
           "unexpected metadata fields")
    ensure(type(metadata["schema"]) is int and metadata["schema"] == 1
           and metadata["source_repo"] == REPOSITORY, "wrong release identity")
    ensure(type(metadata["protocol_min"]) is int and type(metadata["protocol_max"]) is int
           and 1 <= metadata["protocol_min"] <= metadata["protocol_max"] <= 65535
           and (protocol_version is None
                or metadata["protocol_min"] <= protocol_version <= metadata["protocol_max"]),
           "unsupported protocol range")
    ensure(expected_tag is None or metadata["tag"] == expected_tag, "wrong release tag")
    ensure(isinstance(metadata["artifacts"], list) and 0 < len(metadata["artifacts"]) <= 30,
           "invalid artifact list")
    expected_paths = {"release.json", "install.sh"}
    for entry in metadata["artifacts"]:
        required = {"name", "version", "arch", "format", "binary_name", "archive_size",
                    "binary_sha256", "binary_size", "asset_name"}
        ensure(isinstance(entry, dict) and set(entry) in (required, required | {"auxiliary_files"}),
               "unexpected artifact fields")
        path = canonical_path(entry)
        ensure(path not in expected_paths, "duplicate artifact identity")
        expected_paths.add(path)
        ensure(entry["asset_name"] == asset_name(entry), "asset name differs from signed identity")
        ensure(type(entry["archive_size"]) is int and 0 < entry["archive_size"] <= MAX_BINARY
               and type(entry["binary_size"]) is int and 0 < entry["binary_size"] <= MAX_BINARY,
               "invalid signed sizes")
        ensure(isinstance(entry["binary_sha256"], str)
               and re.fullmatch(r"[0-9a-f]{64}", entry["binary_sha256"]), "invalid binary digest")
        ensure(isinstance(entry["binary_name"], str) and SEGMENT.fullmatch(entry["binary_name"])
               and entry["binary_name"] not in (".", ".."), "invalid signed binary name")
        validate_auxiliary_files(entry.get("auxiliary_files", {}), entry["binary_name"], entry["format"])
        if entry["name"] == "tcpquality":
            from tcp_probe_artifact import BINARY, FILES, TOOL_VERSION
            ensure(entry["format"] == "tar.gz" and entry["binary_name"] == BINARY
                   and entry["arch"] in ("amd64", "arm64")
                   and re.fullmatch(re.escape(TOOL_VERSION) + r"-[0-9a-f]{40}-r1", entry["version"])
                   and set(entry.get("auxiliary_files", {})) == FILES - {BINARY},
                   "wrong or incomplete native TCP artifact identity")
        if entry["name"] == "agent":
            binary_name = "sinan-agent.exe" if entry["arch"].startswith("windows-") else "sinan-agent"
            ensure(metadata["tag"] == "agent-v" + entry["version"] and entry["format"] == "raw"
                   and entry["binary_name"] == binary_name, "wrong Agent release identity")
        if entry["format"] == "raw":
            ensure(entry["archive_size"] == entry["binary_size"]
                   and rows.get(path) == entry["binary_sha256"], "raw binary proof mismatch")
    ensure(set(rows) == expected_paths, "unsigned or unused checksum paths")
    ensure(rows["install.sh"] == digest(read_regular(bundle / "install.sh", 256 * 1024)), "installer mismatch")
    return metadata, rows


def verify_bundle(bundle, roots, minisign, expected_tag=None, exact_assets=True):
    bundle = Path(bundle)
    metadata, rows = verify_manifest(bundle, roots, minisign, expected_tag)
    expected_files = {"release.json", "install.sh", "SHA256SUMS", "SHA256SUMS.minisig"}
    for entry in metadata["artifacts"]:
        path = canonical_path(entry)
        expected_files.add(entry["asset_name"])
        data = read_regular(bundle / entry["asset_name"])
        ensure(len(data) == entry["archive_size"] and rows.get(path) == digest(data), "archive mismatch")
        binary = binary_bytes(data, entry["format"], entry["binary_name"], entry.get("auxiliary_files", {}))
        ensure(len(binary) == entry["binary_size"] and digest(binary) == entry["binary_sha256"], "binary mismatch")
        if entry["name"] == "tcpquality":
            from tcp_probe_artifact import archive_files, validate_files
            validate_files(archive_files(data), entry["version"], entry["arch"])
    if exact_assets:
        ensure({p.name for p in bundle.iterdir()} == expected_files, "missing or extra release assets")
    return metadata


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("assemble")
    for argument in ("source", "output", "tag", "agent-version", "runtime-version", "installer"):
        build.add_argument("--" + argument, required=True)
    build.add_argument("--nodequality-version", default=NODEQUALITY_VERSION)
    build.add_argument("--tcp-probe-version", help="opt-in native TCP version with its full source SHA")
    build.add_argument("--arch", action="append", choices=("amd64", "arm64"),
                       help="CI test bundle architectures; production requires both")
    render = commands.add_parser("render-installer")
    for argument in ("template", "agent-unit", "runtime-unit", "output"):
        render.add_argument("--" + argument, required=True)
    verify = commands.add_parser("verify")
    verify.add_argument("--bundle", required=True)
    verify.add_argument("--trusted-keys", required=True)
    verify.add_argument("--minisign", default="minisign")
    verify.add_argument("--tag")
    verify.add_argument("--publication", action="store_true")
    args = parser.parse_args()
    if args.command == "assemble":
        assemble(args)
    elif args.command == "render-installer":
        render_installer(args)
    else:
        roots = load_roots(args.trusted_keys, args.publication)
        verify_bundle(args.bundle, roots, args.minisign, args.tag)
        print("Complete signature, canonical metadata, installer, and every artifact verified.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise SystemExit(f"Release verification failed: {error}") from error
SINAN_BOOTSTRAP_A9D893044F3B931A371AF1C8209993857DFB8FC3920504AE95D03374AB5A7DEF

cat > "$STAGING/tcp_probe_artifact.py" <<'SINAN_BOOTSTRAP_7C9C790035F22EC0554D1B922A5B960571792B991659DDFBF9C0A8E337C0BD0A'
"""Validate the complete, pinned native TCP artifact without executing it."""
import gzip
import hashlib
import io
import json
import re
import struct
import tarfile

TOOL_VERSION = "0.3.0"
BINARY = "sinan-tcp-probe"
FILES = {BINARY, "build-info.json", "LICENSE", "source.tar.gz", "Cargo.lock", "THIRD_PARTY_NOTICES.txt"}
TARGETS = {"amd64": "x86_64-unknown-linux-musl", "arm64": "aarch64-unknown-linux-musl"}
SOURCE_LIMIT = 16 * 1024 * 1024
BUNDLED_MUSL_FILES = ("tools/licenses/bundled-musl.json", "tools/licenses/musl-1.2.5-COPYRIGHT",
                      "tools/licenses/rust-1.98.1-musl-recipe.txt")
BUNDLED_MUSL_IDENTITY = {
    "schema": 1, "version": "1.2.5",
    "source_url": "https://musl.libc.org/releases/musl-1.2.5.tar.gz",
    "source_sha256": "a9a118bbe84d8764da0ea0d28b3ab3fae8477fc7e4085d90102b8596fc7c75e4",
    "copyright_sha256": "f9bc4423732350eb0b3f7ed7e91d530298476f8fec0c6c427a1c04ade22655af",
    "rustc_commit": "48a229ceaefd4985c50990b14116b6d856af0985",
    "rust_recipe_url": "https://raw.githubusercontent.com/rust-lang/rust/48a229ceaefd4985c50990b14116b6d856af0985/src/ci/docker/scripts/musl.sh",
    "rust_recipe_sha256": "2f218a2dc7b7e73509212bfd4319ebddc2ddac7c651fca142c2b29bd7ea0aa38",
    "patches": "CVE-2025-26519: two iconv patches in the pinned Rust recipe; copyright unchanged",
}


def ensure(condition, message):
    if not condition:
        raise ValueError(message)


def digest(value):
    return hashlib.sha256(value).hexdigest()


def artifact_version(commit):
    ensure(isinstance(commit, str) and re.fullmatch(r"[0-9a-f]{40}", commit),
           "TCP source commit must be an actual full lowercase SHA")
    return f"{TOOL_VERSION}-{commit}-r1"


def verify_elf(binary, arch):
    ensure(arch in TARGETS and len(binary) >= 64 and binary[:6] == b"\x7fELF\x02\x01",
           "TCP binary must be a little-endian ELF64")
    ensure(struct.unpack_from("<H", binary, 18)[0] == (62 if arch == "amd64" else 183),
           "TCP binary architecture mismatch")
    offset = struct.unpack_from("<Q", binary, 32)[0]
    size, count = struct.unpack_from("<HH", binary, 54)
    ensure(size == 56 and 0 < count <= 128 and offset + size * count <= len(binary),
           "invalid ELF program headers")
    for index in range(count):
        kind, _, start, _, _, length, _, _ = struct.unpack_from("<IIQQQQQQ", binary, offset + index * size)
        ensure(start + length <= len(binary), "ELF segment exceeds binary")
        ensure(kind != 3, "TCP binary has a dynamic interpreter")
        if kind == 2:
            ensure(length % 16 == 0, "invalid ELF dynamic segment")
            for entry in range(start, start + length, 16):
                tag = struct.unpack_from("<q", binary, entry)[0]
                ensure(tag != 1, "TCP binary has a dynamic library dependency")
                if tag == 0:
                    break


def source_files(data, commit):
    ensure(0 < len(data) <= SOURCE_LIMIT, "source archive size outside permitted range")
    with gzip.GzipFile(fileobj=io.BytesIO(data)) as compressed:
        unpacked = compressed.read(SOURCE_LIMIT + 1)
    ensure(len(unpacked) <= SOURCE_LIMIT, "source archive exceeds unpacked size limit")
    files, consumed = {}, 0
    with tarfile.open(fileobj=io.BytesIO(unpacked), mode="r:") as archive:
        ensure(archive.pax_headers.get("comment") == commit, "source archive commit mismatch")
        for member in archive:
            ensure(len(files) < 10000 and len(member.name) <= 1024
                   and not member.name.startswith("/") and "\\" not in member.name
                   and all(part not in ("", ".", "..") for part in member.name.rstrip("/").split("/")),
                   "unsafe source archive path")
            ensure(member.isdir() or member.isfile(), "source archive must contain ordinary files")
            if member.isdir():
                continue
            ensure(member.name not in files and 0 <= member.size <= SOURCE_LIMIT,
                   "duplicate or oversized source file")
            consumed += member.size
            ensure(consumed <= SOURCE_LIMIT, "source archive exceeds unpacked size limit")
            files[member.name] = archive.extractfile(member).read()
    required = {"Cargo.toml", "Cargo.lock", "LICENSE", "crates/tcp-probe/Cargo.toml",
                "crates/tcp-probe/src/lib.rs", "crates/tcp-probe/src/main.rs",
                "crates/tcp-probe/src/cli.rs", "crates/tcp-probe/src/engine.rs",
                "crates/tcp-probe/src/journal.rs", "crates/tcp-probe/src/model.rs",
                "tools/build-tcp-probe.py", "tools/tcp_probe_artifact.py", "tools/artifact_manifest.py", "tools/tcp_probe_notices.py"}
    ensure(required | set(BUNDLED_MUSL_FILES) <= files.keys(), "source archive is missing the tool or its build recipe")
    return files


def validate_files(files, version, arch):
    import tomllib

    ensure(set(files) == FILES, "TCP artifact must contain its exact complete provenance file set")
    ensure(all(0 < len(value) <= 256 * 1024 * 1024 for value in files.values()), "invalid TCP file size")
    info = json.loads(files["build-info.json"])
    expected = {"schema", "tool", "tool_version", "artifact_version", "source_repo", "source_commit",
                "target", "rustc", "cargo_locked", "source_sha256", "lock_sha256",
                "license_sha256", "binary_sha256", "notices_sha256"}
    ensure(isinstance(info, dict) and set(info) == expected, "invalid TCP build information")
    commit = info["source_commit"]
    ensure(version == artifact_version(commit) and info["artifact_version"] == version,
           "TCP artifact version must include its exact source commit")
    ensure(type(info["schema"]) is int and info["schema"] == 1 and info["tool"] == BINARY
           and info["tool_version"] == TOOL_VERSION and info["source_repo"] == "theLucius7/sinan"
           and info["cargo_locked"] is True and arch in TARGETS and info["target"] == TARGETS[arch]
           and isinstance(info["rustc"], str) and info["rustc"].startswith("rustc ") and len(info["rustc"]) <= 4096,
           "wrong TCP build identity or unlocked build")
    for filename, field in [(BINARY, "binary_sha256"), ("source.tar.gz", "source_sha256"),
                            ("Cargo.lock", "lock_sha256"), ("LICENSE", "license_sha256"),
                            ("THIRD_PARTY_NOTICES.txt", "notices_sha256")]:
        ensure(info[field] == digest(files[filename]), "TCP provenance digest mismatch")
    source = source_files(files["source.tar.gz"], commit)
    ensure(source["Cargo.lock"] == files["Cargo.lock"] and source["LICENSE"] == files["LICENSE"],
           "TCP lockfile or license differs from pinned source")
    workspace = tomllib.loads(source["Cargo.toml"].decode())
    package = tomllib.loads(source["crates/tcp-probe/Cargo.toml"].decode())["package"]
    ensure(workspace["workspace"]["package"]["version"] == TOOL_VERSION
           and workspace["workspace"]["package"]["license"] == "AGPL-3.0-only"
           and package["name"] == BINARY and package["version"] == {"workspace": True}
           and package["license"] == {"workspace": True}, "TCP source version or license mismatch")
    validate_notices(files["THIRD_PARTY_NOTICES.txt"], files["Cargo.lock"], arch, source, info["rustc"])
    verify_elf(files[BINARY], arch)
    return info


def archive_files(data):
    result, consumed = {}, 0
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive:
            ensure(member.name in FILES and member.name not in result and member.isfile()
                   and 0 < member.size <= 256 * 1024 * 1024, "unsafe TCP artifact member")
            consumed += member.size
            ensure(consumed <= 256 * 1024 * 1024, "TCP artifact exceeds unpacked file budget")
            result[member.name] = archive.extractfile(member).read()
    ensure(set(result) == FILES, "TCP artifact is missing provenance files")
    return result


def pack(files):
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode="w", format=tarfile.USTAR_FORMAT) as archive:
            for name in sorted(files):
                member = tarfile.TarInfo(name)
                member.size = len(files[name])
                member.mode = 0o755 if name == BINARY else 0o644
                archive.addfile(member, io.BytesIO(files[name]))
    return output.getvalue()

def bundled_musl(source, rustc_info):
    ensure(set(BUNDLED_MUSL_FILES) <= source.keys(), "bundled musl originals are missing from pinned source")
    identity = json.loads(source[BUNDLED_MUSL_FILES[0]])
    ensure(identity == BUNDLED_MUSL_IDENTITY, "unknown bundled musl source inventory")
    copyright_text, recipe = (source[name] for name in BUNDLED_MUSL_FILES[1:])
    ensure(digest(copyright_text) == identity["copyright_sha256"]
           and digest(recipe) == identity["rust_recipe_sha256"], "bundled musl source originals differ")
    ensure(isinstance(rustc_info, str)
           and [line for line in rustc_info.splitlines() if line.startswith("commit-hash:")]
           == ["commit-hash: " + identity["rustc_commit"]],
           "bundled musl inventory does not cover this rustc commit")
    return dict(name="Rust bundled musl libc", version=identity["version"], notices=[
        dict(path="musl-1.2.5/COPYRIGHT", text=copyright_text.decode()),
        dict(path="rust-1.98.1/musl-recipe.txt", text=recipe.decode()),
        dict(path="bundled-musl/source.json", text=source[BUNDLED_MUSL_FILES[0]].decode()),
    ])


def validate_notices(encoded, lock_bytes, arch, source, rustc_info):
    import tomllib

    ensure(0 < len(encoded) <= 8 * 1024 * 1024, "invalid third-party notice inventory size")
    data = json.loads(encoded)
    ensure(isinstance(data, dict) and set(data) == {"schema", "target", "lock_sha256", "dependencies", "toolchain"}
           and type(data["schema"]) is int and data["schema"] == 1 and data["target"] == TARGETS[arch]
           and data["lock_sha256"] == digest(lock_bytes), "third-party inventory identity mismatch")
    locked = {(p["name"], p["version"], p.get("source")): p for p in tomllib.loads(lock_bytes.decode())["package"]}
    packages = data["dependencies"]
    ensure(isinstance(packages, list) and 0 < len(packages) <= 256, "native dependency notices are missing")
    seen = set()
    def originals(notices):
        ensure(isinstance(notices, list) and 0 < len(notices) <= 128, "license originals are missing")
        names = set()
        for item in notices:
            ensure(isinstance(item, dict) and set(item) == {"path", "text"}
                   and isinstance(item["path"], str) and 0 < len(item["path"]) <= 1024
                   and item["path"] not in names and not item["path"].startswith("/")
                   and all(p not in ("", ".", "..") for p in item["path"].split("/"))
                   and isinstance(item["text"], str) and 0 < len(item["text"]) <= 8 * 1024 * 1024,
                   "invalid license original")
            names.add(item["path"])
    for package in packages:
        ensure(isinstance(package, dict) and set(package) == {"name", "version", "source", "checksum", "license", "notices"},
               "invalid locked dependency notice")
        identity = (package["name"], package["version"], package["source"])
        ensure(all(isinstance(value, str) for value in identity) and identity not in seen
               and identity in locked and locked[identity].get("checksum") == package["checksum"]
               and isinstance(package["license"], str) and package["license"], "notice differs from locked dependency")
        originals(package["notices"])
        if "Unicode" in package["license"]:
            ensure(any("unicode" in n["path"].lower() for n in package["notices"]), "Unicode license original is missing")
        seen.add(identity)
    libraries = data["toolchain"]
    ensure(isinstance(libraries, list) and len(libraries) == 3
           and {p.get("name") for p in libraries} == {"Rust standard library and bundled native libraries", "system musl build tooling", "Rust bundled musl libc"},
           "native library license originals are missing")
    for library in libraries:
        ensure(set(library) == {"name", "version", "notices"} and isinstance(library["version"], str)
               and library["version"], "native library notice identity mismatch")
        originals(library["notices"])
    bundled = next(p for p in libraries if p["name"] == "Rust bundled musl libc")
    ensure(bundled == bundled_musl(source, rustc_info), "bundled musl notice differs from pinned source")
    standard = next(p for p in libraries if p["name"] == "Rust standard library and bundled native libraries")
    ensure(standard["version"] == rustc_info, "Rust notice and binary toolchain versions differ")
    return data
SINAN_BOOTSTRAP_7C9C790035F22EC0554D1B922A5B960571792B991659DDFBF9C0A8E337C0BD0A

cat > "$STAGING/tcp_probe_notices.py" <<'SINAN_BOOTSTRAP_7FE018A190D8AB15F25955182B8FFACA7BA35C543C7E1D562D9D68BE1F1EBA77'
"""Collect original notices from checksum-locked native Cargo packages and the toolchain."""
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile
import tomllib

from tcp_probe_artifact import BINARY, TARGETS, BUNDLED_MUSL_FILES, bundled_musl, digest, ensure

LIMIT = 8 * 1024 * 1024


def capture(command):
    return subprocess.run(command, check=True, capture_output=True, timeout=60).stdout


def collect(repository, arch):
    target = TARGETS[arch]
    rustc_info = capture(["rustc", "-vV"]).decode()
    pinned = {name: (repository / name).read_bytes() for name in BUNDLED_MUSL_FILES}
    libc = bundled_musl(pinned, rustc_info)
    metadata = json.loads(capture(["cargo", "metadata", "--locked", "--format-version", "1",
                                  "--filter-platform", target, "--manifest-path", str(repository / "Cargo.toml")]))
    # Select normal/build edges for this package, not features unified across all workspace roots.
    tree = capture(["cargo", "tree", "--locked", "--package", BINARY, "--target", target,
                    "--edges", "normal,build", "--prefix", "none", "--format", "{p}",
                    "--manifest-path", str(repository / "Cargo.toml")]).decode()
    selected = set()
    for line in tree.splitlines():
        match = re.match(r"^([A-Za-z0-9_-]+) v([^ ]+)(?: |$)", line)
        ensure(match, "cannot parse locked native dependency selection")
        selected.add(match.groups())
    lock_bytes = (repository / "Cargo.lock").read_bytes()
    lock = {(p["name"], p["version"], p.get("source")): p for p in tomllib.loads(lock_bytes.decode())["package"]}
    dependencies = []
    for package in metadata["packages"]:
        identity = (package["name"], package["version"])
        if identity not in selected or package["name"] == BINARY:
            continue
        source = package.get("source")
        ensure(source and source.startswith("registry+"), "native dependencies require locked registry checksums")
        entry = lock.get((*identity, source))
        ensure(entry and re.fullmatch(r"[0-9a-f]{64}", entry.get("checksum", "")),
               "native dependency is absent from the locked checksum inventory")
        directory = Path(package["manifest_path"]).parent
        crate = directory.parents[2] / "cache" / directory.parent.name / (directory.name + ".crate")
        ensure(crate.is_file() and not crate.is_symlink(), "locked dependency source archive is missing")
        packed = crate.read_bytes()
        ensure(0 < len(packed) <= 16 * 1024 * 1024 and digest(packed) == entry["checksum"],
               "locked dependency source checksum mismatch")
        notices, count, total = [], 0, 0
        with tarfile.open(fileobj=io.BytesIO(packed), mode="r:gz") as archive:
            for member in archive:
                count += 1
                ensure(count <= 10000 and (member.isdir() or member.isfile()),
                       "unsafe locked dependency source")
                if member.isdir():
                    continue
                parts = member.name.split("/")
                ensure(parts[0] == directory.name and len(parts) > 1 and all(p not in ("", ".", "..") for p in parts),
                       "unsafe locked dependency source path")
                relative = "/".join(parts[1:])
                total += member.size
                ensure(0 <= member.size <= 16 * 1024 * 1024 and total <= 64 * 1024 * 1024,
                       "locked dependency source exceeds size limits")
                original = archive.extractfile(member).read()
                installed = directory / relative
                ensure(installed.is_file() and not installed.is_symlink() and installed.read_bytes() == original,
                       "installed dependency source differs from its locked archive")
                if re.match(r"^(LICENSE|LICENCE|COPYING|COPYRIGHT|NOTICE)(?:[._-]|$)", parts[-1], re.IGNORECASE):
                    ensure(0 < len(original) <= 512 * 1024, "dependency notice exceeds permitted size")
                    notices.append(dict(path=relative, text=original.decode("utf-8")))
        license_id = package.get("license")
        ensure(isinstance(license_id, str) and license_id and notices, "dependency license originals are missing")
        if "Unicode" in license_id:
            ensure(any("unicode" in n["path"].lower() for n in notices), "Unicode license original is missing")
        dependencies.append(dict(name=identity[0], version=identity[1], source=source,
                                 checksum=entry["checksum"], license=license_id, notices=sorted(notices, key=lambda n:n["path"])))
    ensure({(p["name"], p["version"]) for p in dependencies} == selected - {(BINARY, "0.3.0")},
           "native dependency selection is incomplete or ambiguous")
    sysroot = Path(capture(["rustc", "--print", "sysroot"]).decode().strip())
    rust_docs = sysroot / "share/doc/rust"
    originals = [rust_docs / "COPYRIGHT-library.html", *(rust_docs / "licenses").glob("*")]
    rust_notices = []
    for path in sorted(originals):
        ensure(path.is_file() and not path.is_symlink(), "Rust library notice original is missing")
        text = path.read_text()
        ensure(0 < len(text.encode()) <= LIMIT, "Rust library notice exceeds permitted size")
        rust_notices.append(dict(path=str(path.relative_to(rust_docs)), text=text))
    ensure(len(rust_notices) >= 2, "Rust library license originals are missing")
    musl_path = Path("/usr/share/doc/musl/copyright")
    ensure(musl_path.is_file() and not musl_path.is_symlink(), "system musl copyright original is missing")
    musl_version = capture(["dpkg-query", "-W", "-f", "${Version}", "musl"]).decode().strip()
    result = dict(schema=1, target=target, lock_sha256=digest(lock_bytes),
                  dependencies=sorted(dependencies, key=lambda p:(p["name"], p["version"], p["source"])),
                  toolchain=[
                      dict(name="Rust standard library and bundled native libraries",
                           version=rustc_info, notices=rust_notices),
                      dict(name="system musl build tooling", version=musl_version,
                           notices=[dict(path="musl/copyright", text=musl_path.read_text())]),
                      libc,
                  ])
    encoded = (json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()
    ensure(len(encoded) <= LIMIT, "third-party notice inventory exceeds size limit")
    return encoded
SINAN_BOOTSTRAP_7FE018A190D8AB15F25955182B8FFACA7BA35C543C7E1D562D9D68BE1F1EBA77

cat > "$STAGING/artifact_manifest.py" <<'SINAN_BOOTSTRAP_8B75C06855E5CA6222579D9DD5B14185451D81B0CB68219B1B2540F92415324D'
"""Publish immutable flat artifacts with a verified checksum manifest."""
import hashlib
import os
from pathlib import Path
import re
import uuid

KEY = re.compile(r"(?:amd64|arm64|linux-(?:gnu|musl)-(?:amd64|arm64)|macos-arm64|(?:freebsd|windows)-(?:amd64|arm64))\Z")


def publish(directory, name, payload):
    directory = Path(directory)
    if not KEY.fullmatch(name) or directory.is_symlink():
        raise ValueError("unsafe artifact target or directory")
    directory.mkdir(parents=True, exist_ok=True)
    manifest = directory / "SHA256SUMS"
    target = directory / name
    if target.exists() or target.is_symlink():
        raise FileExistsError(target)
    checksums = {}
    if manifest.is_symlink():
        raise ValueError("checksum manifest cannot be a symlink")
    if manifest.exists():
        for line in manifest.read_text(encoding="utf-8").splitlines():
            parts = line.split()
            if len(parts) != 2 or not re.fullmatch(r"[0-9a-fA-F]{64}", parts[0]) or not KEY.fullmatch(parts[1]) or parts[1] in checksums:
                raise ValueError("invalid checksum manifest")
            checksums[parts[1]] = parts[0].lower()
    for path in directory.iterdir():
        if KEY.fullmatch(path.name):
            if path.is_symlink() or not path.is_file() or checksums.get(path.name) != hashlib.sha256(path.read_bytes()).hexdigest():
                raise ValueError("existing artifact lacks a valid checksum")
    for key in checksums:
        if not (directory / key).is_file():
            raise ValueError("checksum references a missing artifact")
    temporary = directory / f".artifact-{uuid.uuid4()}"
    manifest_temporary = directory / f".checksums-{uuid.uuid4()}"
    try:
        with temporary.open("xb") as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        os.link(temporary, target)
        checksums[name] = hashlib.sha256(payload).hexdigest()
        with manifest_temporary.open("x", encoding="utf-8", newline="\n") as output:
            output.write("".join(f"{digest}  {key}\n" for key, digest in sorted(checksums.items())))
            output.flush()
            os.fsync(output.fileno())
        os.replace(manifest_temporary, manifest)
    finally:
        temporary.unlink(missing_ok=True)
        manifest_temporary.unlink(missing_ok=True)
    return target
SINAN_BOOTSTRAP_8B75C06855E5CA6222579D9DD5B14185451D81B0CB68219B1B2540F92415324D

cat > "$STAGING/public-keys.json" <<'SINAN_BOOTSTRAP_51121348A57E37D3396225828114D19A3F62EDB0AAD0F6157E7FBCBEBF56B576'
["RWS4aZYmyBmwROpGKjfADJqNedYCNRhlg0+UoIBjQHxXZxYL7XMlkGJN"]
SINAN_BOOTSTRAP_51121348A57E37D3396225828114D19A3F62EDB0AAD0F6157E7FBCBEBF56B576

cat > "$STAGING/trusted-install.sh" <<'SINAN_BOOTSTRAP_C6C95142E68A89AA786F475B3209016DAF80A8320F115DB3215FB7D94B62D439'
#!/bin/sh
# Static signed release installer. Invoke only after independent verification.
# SINAN_BOOTSTRAP_AGENT_SOURCE=preloaded-github-v1
set -eu
umask 027
BUNDLE=
PANEL=
VERSION=
TARGET=
TOKEN_FILE=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --bundle) BUNDLE=$2 ;;
    --panel) PANEL=$2 ;;
    --version) VERSION=$2 ;;
    --target) TARGET=$2 ;;
    --token-file) TOKEN_FILE=$2 ;;
    *) echo '未知安装参数' >&2; exit 2 ;;
  esac
  shift 2
done
[ "$(id -u)" = 0 ] || { echo '请以 root 运行' >&2; exit 1; }
[ -n "$BUNDLE" ] && [ -n "$PANEL" ] && [ -n "$VERSION" ] && [ -n "$TOKEN_FILE" ] || exit 2
[ "$(uname -s)" = Linux ] || { echo '此签名安装器需要 Linux；原生平台须独立核对签名目录后执行 install-services' >&2; exit 1; }
if [ -d /run/systemd/system ] && command -v systemctl >/dev/null; then
  INIT=systemd
elif [ -f /run/openrc/softlevel ] && command -v rc-service >/dev/null && command -v rc-update >/dev/null; then
  INIT=openrc
  for tool in openrc-run supervise-daemon; do command -v "$tool" >/dev/null || exit 1; done
  SUPERVISOR_HELP=$(RC_SVCNAME=sinan-agent supervise-daemon sinan-agent --help 2>&1)
  case "$SUPERVISOR_HELP" in *--capabilities*) ;; *) echo 'OpenRC 缺少 libcap 支持' >&2; exit 1 ;; esac
  case "$SUPERVISOR_HELP" in *--no-new-privs*) ;; *) echo 'OpenRC 缺少 no_new_privs 支持' >&2; exit 1 ;; esac
else
  echo '需要运行中的 Linux systemd 或 OpenRC' >&2; exit 1
fi
for tool in python3 install getent cmp mv seq; do
  command -v "$tool" >/dev/null || { echo "缺少工具: $tool" >&2; exit 1; }
done
case "$(uname -m)" in x86_64|amd64) ARCH=amd64 ;; aarch64|arm64) ARCH=arm64 ;; *) exit 1 ;; esac
[ -n "$TARGET" ] || TARGET=$ARCH
case "$TARGET" in "$ARCH"|"linux-musl-$ARCH"|"linux-gnu-$ARCH") ;; *) echo '所选 Agent 平台与本机架构不兼容' >&2; exit 1 ;; esac
python3 -I - "$VERSION" <<'PY'
#!/usr/bin/env python3
"""Read-only recovery gate for independently signed Agent versions before 0.3.1."""

import errno
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import time
import urllib.parse
import uuid

SEGMENT = re.compile(r"[0-9A-Za-z][0-9A-Za-z.+_-]{0,127}\Z")
LEGACY_NODEQUALITY_VERSION = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2"


def ensure(condition, message):
    if not condition:
        raise ValueError(message)


def protected(path):
    for part in (path,) + tuple(path.parents):
        properties = part.lstat()
        ensure(not part.is_symlink() and properties.st_uid == 0 and properties.st_mode & 0o022 == 0,
               "旧 Agent 状态路径必须由 root 保护，安装未切换")


def regular(path, limit):
    protected(path)
    ensure(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= limit,
           "旧 Agent 状态必须是有界普通文件，安装未切换")
    return path.read_bytes()


def quiescent(saved, allow_missing=False):
    """An old process must not create Preparing after the read-only preflight."""
    environment = os.environ.copy()
    environment["LC_ALL"] = "C"
    try:
        if Path("/run/systemd/system").is_dir():
            result = subprocess.run(["systemctl", "show", "sinan-agent.service", "--property=LoadState",
                                     "--property=ActiveState", "--property=SubState", "--property=MainPID"],
                                    capture_output=True, env=environment, check=False, timeout=3)
            ensure(result.returncode == 0 and len(result.stdout) <= 4096,
                   "无法确认旧 Agent 服务停止，请先执行受控维护，安装未切换")
            properties = {}
            for line in result.stdout.decode("ascii").splitlines():
                key, separator, value = line.partition("=")
                ensure(separator and key not in properties, "旧 Agent 服务状态无效，安装未切换")
                properties[key] = value
            ensure(set(properties) == {"LoadState", "ActiveState", "SubState", "MainPID"}
                   and properties["LoadState"] in (("loaded", "not-found") if allow_missing else ("loaded",))
                   and properties["ActiveState"] == "inactive" and properties["SubState"] == "dead"
                   and properties["MainPID"] == "0", "旧 Agent 仍可能运行，请先执行受控维护，安装未切换")
        elif Path("/run/openrc/softlevel").is_file():
            exists = subprocess.run(["rc-service", "--exists", "sinan-agent"], capture_output=True,
                                    env=environment, check=False, timeout=3)
            status = subprocess.run(["rc-service", "sinan-agent", "status"], capture_output=True,
                                    env=environment, check=False, timeout=3)
            ensure((exists.returncode == 0 and status.returncode == 3)
                   or (allow_missing and exists.returncode == 1 and status.returncode == 1),
                   "无法确认旧 Agent 已停止，请先执行受控维护，安装未切换")
        else:
            raise ValueError("无法确认旧 Agent 服务状态，请先执行受控维护，安装未切换")
    except (OSError, subprocess.TimeoutExpired, UnicodeError):
        raise ValueError("无法确认旧 Agent 服务停止，请先执行受控维护，安装未切换") from None
    endpoint = saved.get("status_socket", "/run/sinan/agent.sock")
    ensure(isinstance(endpoint, str) and Path(endpoint).is_absolute()
           and ".." not in Path(endpoint).parts and not any(ord(character) < 32 for character in endpoint),
           "旧 Agent 状态端点无效，安装未切换")
    connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    connection.settimeout(0.5)
    try:
        connection.connect(endpoint)
    except OSError as error:
        ensure(error.errno in (errno.ENOENT, errno.ECONNREFUSED),
               "无法确认旧 Agent 状态端点停止，安装未切换")
    else:
        raise ValueError("旧 Agent 状态端点仍在运行，请先执行受控维护，安装未切换")
    finally:
        connection.close()


def unique_object(pairs):
    value = {}
    for key, entry in pairs:
        ensure(key not in value, "旧诊断状态字段重复，安装未切换")
        value[key] = entry
    return value


def options_valid(value):
    return (isinstance(value, dict) and len(value) <= 32
            and all(isinstance(key, str) and len(key) <= 128 and isinstance(entry, str)
                    and len(entry) <= 1024 for key, entry in value.items()))


def optional_fields_valid(value):
    return ((value.get("expires_at") is None or type(value["expires_at"]) is int
             and -(2 ** 63) <= value["expires_at"] < 2 ** 63)
            and all(value.get(key) is None or isinstance(value[key], str) and len(value[key]) <= 4096
                    for key in ("start_error", "protection_stop_reason")))


def checkpoint_safe(encoded):
    try:
        checkpoint = json.loads(encoded, object_pairs_hook=unique_object,
                                parse_constant=lambda _value: (_ for _ in ()).throw(ValueError()))
    except (ValueError, RecursionError):
        raise ValueError("旧诊断状态无法安全解析，安装未切换") from None
    if checkpoint is None:
        return
    ensure(isinstance(checkpoint, dict) and len(checkpoint) == 1
           and next(iter(checkpoint)) in ("Preparing", "Started"), "旧诊断检查点未知，安装未切换")
    phase = next(iter(checkpoint))
    value = checkpoint[phase]
    ensure(isinstance(value, dict), "旧诊断检查点损坏，安装未切换")
    if phase == "Started":
        ensure({"spec", "service", "plugin", "started_at"} <= set(value)
               and set(value) <= {"spec", "service", "plugin", "started_at", "start_error",
                                  "expires_at", "protection_stop_reason", "environment"}
               and isinstance(value["spec"], dict) and isinstance(value["service"], dict)
               and value["plugin"] == "nodequality" and type(value["started_at"]) is int
               and 0 <= value["started_at"] < 2 ** 64 and optional_fields_valid(value)
               and (value.get("environment") is None or isinstance(value["environment"], dict)),
               "旧已启动检查点损坏，安装未切换")
        spec, service = value["spec"], value["service"]
        ensure(set(spec) == {"id", "version", "binary_path", "job_dir", "timeout_secs", "options"}
               and {"unit", "program", "args", "working_directory", "timeout_secs"} <= set(service)
               and set(service) <= {"unit", "program", "args", "working_directory", "timeout_secs",
                                    "memory_max", "tasks_max", "cpu_weight", "io_weight", "oom_score_adjust"}
               and isinstance(spec.get("version"), str) and SEGMENT.fullmatch(spec["version"])
               and isinstance(spec.get("id"), str) and len(spec["id"]) == 36
               and isinstance(spec.get("binary_path"), str) and Path(spec["binary_path"]).is_absolute()
               and isinstance(spec.get("job_dir"), str) and Path(spec["job_dir"]).is_absolute()
               and type(spec.get("timeout_secs")) is int and 1 <= spec["timeout_secs"] <= 3600
               and type(service.get("timeout_secs")) is int and service["timeout_secs"] == spec["timeout_secs"]
               and options_valid(spec.get("options")) and isinstance(service.get("args"), list)
               and len(service["args"]) <= 64
               and all(isinstance(argument, str) and len(argument) <= 4096 for argument in service["args"])
               and service.get("working_directory") == spec["job_dir"]
               and service.get("program") == spec["binary_path"]
               and service.get("unit") == "sinan-diagnostic-" + spec["id"] + ".service",
               "旧已启动检查点版本或执行身份无效，安装未切换")
        ensure(all(type(service[key]) is int for key in
                   {"memory_max", "tasks_max", "cpu_weight", "io_weight", "oom_score_adjust"} & set(service)),
               "旧已启动检查点资源形状无效，安装未切换")
        ensure(spec["version"] == LEGACY_NODEQUALITY_VERSION
               and set(spec["options"]) <= {"ip_version", "network_mode", "upload_report"}
               and spec["options"].get("ip_version", "both") in ("both", "ipv4", "ipv6")
               and spec["options"].get("network_mode", "low") in ("low", "normal")
               and spec["options"].get("upload_report", "false") in ("true", "false"),
               "旧 Agent 无法按原版本回收此已启动任务；请保留状态并使用兼容的新签名 Agent，安装未切换")
        try:
            uuid.UUID(spec["id"])
        except (ValueError, TypeError, AttributeError):
            raise ValueError("旧已启动检查点身份无效，安装未切换") from None
        # Existing compiled-root verify-cache validates the exact saved executable proof;
        # the old Agent only observes Started instead of executing it again.
        return
    ensure({"id", "version", "artifact", "timeout_secs"} <= set(value)
           and set(value) <= {"id", "version", "artifact", "timeout_secs", "plugin", "options",
                              "expires_at", "resource_budget"}
           and isinstance(value["version"], str) and SEGMENT.fullmatch(value["version"])
           and type(value["timeout_secs"]) is int and 1 <= value["timeout_secs"] <= 3600
           and isinstance(value["artifact"], dict) and optional_fields_valid(value),
           "旧排队检查点损坏，安装未切换")
    try:
        ensure(isinstance(value["id"], str) and len(value["id"]) == 36, "invalid checkpoint UUID")
        uuid.UUID(value["id"])
    except (ValueError, TypeError, AttributeError):
        raise ValueError("旧排队检查点身份无效，安装未切换") from None
    artifact = value["artifact"]
    proof = artifact.get("proof")
    ensure(set(artifact) == {"url", "sha256", "proof"} and isinstance(artifact["url"], str)
           and 0 < len(artifact["url"]) <= 8192 and isinstance(artifact["sha256"], str)
           and re.fullmatch(r"[0-9a-f]{64}", artifact["sha256"])
           and isinstance(proof, dict) and set(proof) == {"metadata_json", "checksums", "signature"}
           and all(isinstance(proof[key], str) and 0 < len(proof[key]) <= limit
                   for key, limit in (("metadata_json", 32768), ("checksums", 8192), ("signature", 16384))),
           "旧排队检查点签名证明形状无效，安装未切换")
    options = value.get("options", {})
    budget = value.get("resource_budget")
    ensure(budget is None or isinstance(budget, dict)
           and set(budget) == {"memory_max", "tasks_max", "cpu_weight", "io_weight", "oom_score_adjust"}
           and all(type(entry) is int for entry in budget.values()),
           "旧排队检查点资源形状无效，安装未切换")
    ensure(options_valid(options) and value.get("plugin", "nodequality") == "nodequality",
           "旧排队检查点插件或参数未知，安装未切换")
    mode = options.get("mode", "full")
    ensure(mode in ("full", "daily"), "旧排队检查点模式未知，安装未切换")
    ensure(mode != "full", "旧 Agent 存在尚未启动的完整验机任务；恢复门禁拒绝，原状态保留，安装未切换")


def _preflight(version, configuration):
    ensure(isinstance(version, str) and re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?", version),
           "旧 Agent 版本无法安全确认")
    if tuple(int(part) for part in version.split("-")[0].split("+")[0].split(".")) > (0, 3, 0):
        return
    configuration = Path(configuration)
    database, saved = Path("/var/lib/sinan/core/state.db"), {}
    if configuration.exists() or configuration.is_symlink():
        try:
            import tomllib
        except ImportError:
            raise ValueError("旧 Agent 恢复预检需要 Python 3.11 或更新版本，安装未切换") from None
        try:
            saved = tomllib.loads(regular(configuration, 1048576).decode("utf-8"))
        except (ValueError, UnicodeError):
            raise ValueError("旧 Agent 配置无法安全解析，安装未切换") from None
        state_path = saved.get("state_db", str(database))
        ensure(isinstance(state_path, str) and Path(state_path).is_absolute()
               and not any(ord(character) < 32 for character in state_path)
               and ".." not in Path(state_path).parts, "旧 Agent 状态位置无法安全确认，安装未切换")
        database = Path(state_path)
        quiescent(saved, allow_missing=not database.exists())
    if not database.exists() and not database.is_symlink():
        return
    if not (configuration.exists() or configuration.is_symlink()):
        raise ValueError("旧 Agent 状态存在但配置缺失，安装未切换")
    protected(database)
    ensure(database.is_file(), "旧 Agent 状态必须是普通文件，安装未切换")
    wal, shm = (Path(str(database) + suffix) for suffix in ("-wal", "-shm"))
    for sidecar in (wal, shm):
        if sidecar.exists() or sidecar.is_symlink():
            protected(sidecar)
            ensure(sidecar.is_file(), "旧 Agent 状态附属文件无效，安装未切换")
    ensure(not wal.exists() or wal.stat().st_size == 0 or shm.exists(),
           "旧 Agent WAL 缺少共享内存，拒绝创建恢复文件，安装未切换")
    try:
        import sqlite3
    except ImportError:
        raise ValueError("旧 Agent 状态预检需要 Python 标准库 SQLite 支持，安装未切换") from None
    status = lambda: [(path.exists(), path.stat().st_ino, path.stat().st_size, path.stat().st_mtime_ns)
                      if path.exists() else (False,) for path in (database, wal, shm)]
    before, deadline = status(), time.monotonic() + 1
    connection = None
    try:
        uri = "file:" + urllib.parse.quote(str(database), safe="/") + "?mode=ro"
        connection = sqlite3.connect(uri, uri=True, timeout=1)
        connection.set_progress_handler(lambda: int(time.monotonic() >= deadline), 1000)
        rows = connection.execute(
            "SELECT substr(value,1,1048577),typeof(value) FROM kv WHERE key='diagnostics:active' LIMIT 2").fetchall()
    except sqlite3.Error:
        raise ValueError("旧诊断状态无法只读确认，安装未切换") from None
    finally:
        if connection is not None:
            connection.close()
    ensure(status() == before, "旧诊断状态在预检期间变化，安装未切换")
    ensure(len(rows) <= 1, "旧诊断状态重复，安装未切换")
    if rows:
        encoded, kind = rows[0]
        ensure(kind == "text" and isinstance(encoded, str) and len(encoded.encode("utf-8")) <= 1048576,
               "旧诊断状态不是有界 JSON，安装未切换")
        checkpoint_safe(encoded)


def preflight(version, configuration=Path("/etc/sinan/agent.toml")):
    try:
        _preflight(version, configuration)
    except OSError:
        raise ValueError("旧状态路径无法安全读取，安装未切换") from None


if __name__ == "__main__":
    try:
        ensure(len(sys.argv) == 2, "旧 Agent 预检参数无效")
        preflight(sys.argv[1])
    except ValueError as error:
        raise SystemExit(f"Legacy Agent refused: {error}") from None
    except OSError:
        raise SystemExit("Legacy Agent refused: 旧状态路径无法安全读取，安装未切换") from None
PY
DOWNLOAD=$(mktemp -d)
STAGE=
PREVIOUS=$(readlink /opt/sinan/core/current 2>/dev/null || true)
ACTIVATING=0
COMPLETED=0
CONFIGURATION_CHANGED=0
if [ -f /etc/sinan/agent.toml ]; then cp -p /etc/sinan/agent.toml "$DOWNLOAD/agent.toml.previous"; fi
if [ "$INIT" = systemd ] && [ -f /etc/systemd/system/sinan-agent.service ]; then cp -p /etc/systemd/system/sinan-agent.service "$DOWNLOAD/agent.service.previous"; fi
if [ "$INIT" = openrc ] && [ -f /etc/init.d/sinan-agent ]; then cp -p /etc/init.d/sinan-agent "$DOWNLOAD/agent.openrc.previous"; fi
cleanup() {
  result=$?
  trap - EXIT HUP INT TERM
  if [ "$CONFIGURATION_CHANGED" = 1 ] && [ "$ACTIVATING" != 1 ] && [ "$COMPLETED" != 1 ] && [ -f "$DOWNLOAD/agent.toml.previous" ]; then
    cp -p "$DOWNLOAD/agent.toml.previous" /etc/sinan/agent.toml
  fi
  if [ "$ACTIVATING" = 1 ] && [ "$COMPLETED" != 1 ]; then
    printf '%s\n' '新 Agent 未通过启动检查，正在恢复上一版本。' >&2
    if [ "$INIT" = systemd ]; then systemctl stop sinan-agent.service || true
    else rc-service sinan-agent stop || true; fi
    if [ -f "$DOWNLOAD/agent.toml.previous" ]; then cp -p "$DOWNLOAD/agent.toml.previous" /etc/sinan/agent.toml; fi
    if [ -f "$DOWNLOAD/agent.service.previous" ]; then cp -p "$DOWNLOAD/agent.service.previous" /etc/systemd/system/sinan-agent.service; systemctl daemon-reload; fi
    if [ -f "$DOWNLOAD/agent.openrc.previous" ]; then cp -p "$DOWNLOAD/agent.openrc.previous" /etc/init.d/sinan-agent; rc-update --update; fi
    if [ -n "$PREVIOUS" ]; then
      ln -s "$PREVIOUS" "/opt/sinan/core/recover.$$"
      mv -Tf "/opt/sinan/core/recover.$$" /opt/sinan/core/current
      if [ "$INIT" = systemd ]; then systemctl restart sinan-agent.service || true
      else rc-service sinan-agent restart || true; fi
    else
      rm -f /opt/sinan/core/current
    fi
  fi
  rm -rf "$DOWNLOAD"
  [ -z "$STAGE" ] || rm -rf "$STAGE"
  exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
TOKEN=$(cat "$TOKEN_FILE")
# Bootstrap downloaded this asset from GitHub and verified the signed proof.
python3 -I - "$BUNDLE" "$VERSION" "$ARCH" "$DOWNLOAD/sinan-agent" "$PANEL" "$TOKEN_FILE" "$TARGET" <<'PY'
import hashlib, ipaddress, json, pathlib, platform, re, subprocess, sys, urllib.parse

def ensure(condition, message):
    if not condition:
        raise ValueError(message)

def ordinary(path, limit):
    ensure(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= limit,
           '安装证明必须是有界普通文件')
    return path.read_bytes()

def protected_existing_parents(path):
    while not path.exists() and not path.is_symlink():
        path = path.parent
    for current in (path, *path.parents):
        properties = current.lstat()
        ensure(current.is_dir() and not current.is_symlink() and properties.st_uid == 0
               and properties.st_mode & 0o022 == 0,
               'Agent 执行目录与父目录必须由 root 保护，不能含符号链接')

root, version, arch, asset = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3], pathlib.Path(sys.argv[4])
panel, token_file = sys.argv[5], pathlib.Path(sys.argv[6])
target = sys.argv[7]
protected_existing_parents(pathlib.Path('/opt/sinan/core'))
ensure(re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?', version), '无效 Agent 版本')
ensure(arch in ('amd64', 'arm64'), '无效 Agent 架构')
ensure(target in (arch, 'linux-musl-' + arch, 'linux-gnu-' + arch), 'Agent 平台与本机 CPU 不匹配')
if target.startswith('linux-gnu-'):
    libc = platform.libc_ver()[0].lower()
    if libc not in ('glibc', 'gnu'):
        try:
            result = subprocess.run(['ldd', '--version'], capture_output=True, check=False, timeout=10)
            description = (result.stdout + result.stderr).decode('utf-8', errors='replace').lower()
        except (OSError, subprocess.TimeoutExpired):
            description = ''
        libc = 'gnu' if 'glibc' in description or 'gnu libc' in description else ''
    ensure(libc in ('glibc', 'gnu'), 'GNU Agent 需要 GNU/glibc 主机，不能在 musl 上安装')
origin = urllib.parse.urlsplit(panel)
ensure(not any(ord(character) < 32 or ord(character) == 127 for character in panel)
       and origin.scheme in ('http', 'https') and origin.hostname
       and not origin.username and not origin.password and origin.path in ('', '/')
       and not origin.query and not origin.fragment
       and (origin.port is None or 0 < origin.port <= 65535), '面板地址必须是有效同源地址')
if origin.scheme == 'http':
    try:
        address = ipaddress.ip_address(origin.hostname)
        address = getattr(address, 'ipv4_mapped', None) or address
        loopback = address.is_loopback
    except ValueError:
        loopback = origin.hostname == 'localhost'
    ensure(loopback, 'HTTP 面板仅允许回环地址，请使用 HTTPS')
metadata_bytes = ordinary(root / 'release.json', 32768)
metadata = json.loads(metadata_bytes)
ensure(type(metadata['schema']) is int and metadata['schema'] == 1
       and metadata['source_repo'] == 'theLucius7/sinan', '发布身份不匹配')
ensure(metadata['tag'] == 'agent-v' + version, '发布标签不匹配')
matches = [item for item in metadata['artifacts'] if
           (item['name'], item['version'], item['arch']) == ('agent', version, target)]
ensure(len(matches) == 1, '发布缺失或重复目标 Agent')
item = matches[0]
expected_name = 'agent-' + version + ('-linux-musl-' + arch if target == arch else '-' + target)
ensure(item['format'] == 'raw' and item['binary_name'] == 'sinan-agent'
       and item['asset_name'] == expected_name, 'Agent 制品身份不匹配')
expected_size = item['archive_size']
ensure(type(expected_size) is int and 0 < expected_size <= 256 * 1024 * 1024
       and type(item['binary_size']) is int and item['binary_size'] == expected_size,
       '已签 Agent 大小无效')
ensure(isinstance(item['binary_sha256'], str)
       and re.fullmatch(r'[0-9a-f]{64}', item['binary_sha256']), '已签 Agent 摘要无效')
checksums = {}
for line in ordinary(root / 'SHA256SUMS', 8192).decode('utf-8').splitlines():
    match = re.fullmatch(r'([0-9a-f]{64})  ([0-9A-Za-z.+_/-]+)', line)
    ensure(match is not None and match[2] not in checksums, '发布清单格式错误或重复')
    checksums[match[2]] = match[1]
ensure(checksums.get('release.json') == hashlib.sha256(metadata_bytes).hexdigest(), '已签 metadata 摘要不匹配')
ensure(checksums.get('agent/' + version + '/' + target) == item['binary_sha256'], '已签 Agent 摘要不匹配')
source = root / expected_name
payload = ordinary(source, expected_size)
ensure(len(payload) == expected_size, 'Agent 长度不匹配')
ensure(hashlib.sha256(payload).hexdigest() == item['binary_sha256'], 'Agent 摘要不匹配，拒绝执行')
with asset.open('xb') as output:
    output.write(payload)
PY
install -d -m 0755 -o root -g root /opt/sinan /opt/sinan/core
STAGE=$(mktemp -d /opt/sinan/core/.bootstrap.XXXXXX)
install -d -m 0755 "$STAGE/$VERSION"
install -m 0755 "$DOWNLOAD/sinan-agent" "$STAGE/$VERSION/sinan-agent"
for proof in release.json SHA256SUMS SHA256SUMS.minisig; do
  install -m 0644 "$BUNDLE/$proof" "$STAGE/$VERSION/$proof"
done
# Compiled Agent roots must accept the proof before enrollment or activation.
"$STAGE/$VERSION/sinan-agent" verify-installed --binary "$STAGE/$VERSION/sinan-agent" --name agent --format raw
# Reject unverifiable legacy caches before touching identity, units or current links.
if [ -e /etc/sinan/agent.toml ] || [ -L /etc/sinan/agent.toml ]; then
  [ -f /etc/sinan/agent.toml ] && [ ! -L /etc/sinan/agent.toml ] || {
    echo '已有 Agent 配置必须是普通文件，安装未切换' >&2; exit 1;
  }
  "$STAGE/$VERSION/sinan-agent" --config /etc/sinan/agent.toml verify-cache || {
    echo '已有运行时或未完成操作未通过签名预检，请按发布文档迁移；安装未切换' >&2
    exit 1
  }
else
  python3 -I - "$PANEL" <<'PY'
import ipaddress, os, re, sys, urllib.parse
from pathlib import Path
paths = [Path('/var/lib/sinan/core/state.db' + suffix) for suffix in ('', '-wal', '-shm')]
paths.append(Path('/opt/sinan/core/current'))
plugins = Path('/opt/sinan/plugins')
if plugins.exists():
    paths.extend(plugins.glob('*/current'))
for path in paths:
    if path.exists() or path.is_symlink():
        raise SystemExit('已有安装状态但配置缺失，签名预检无法确定引用；安装未切换')

def protected(path):
    for current in (path, *path.parents):
        info = current.lstat()
        if current.is_symlink() or info.st_uid != 0 or info.st_mode & 0o022:
            raise SystemExit('首次接入身份与父目录必须由 root 保护')
        if current != path and not current.is_dir():
            raise SystemExit('首次接入身份父路径必须是普通目录')

def ordinary(path, limit):
    protected(path)
    if not path.is_file() or not 0 < path.stat().st_size <= limit:
        raise SystemExit('首次接入身份必须是有界普通文件')
    return path.read_bytes()

def origin(value):
    parsed = urllib.parse.urlsplit(value)
    if (any(ord(c) < 32 or ord(c) == 127 for c in value) or parsed.scheme not in ('http', 'https')
            or not parsed.hostname or parsed.username or parsed.password or parsed.path not in ('', '/')
            or parsed.query or parsed.fragment or parsed.port is not None and not 0 < parsed.port <= 65535):
        raise SystemExit('首次接入面板地址无效')
    hostname = parsed.hostname.lower()
    try:
        address = ipaddress.ip_address(hostname)
        hostname = address.compressed
        loopback = (getattr(address, 'ipv4_mapped', None) or address).is_loopback
    except ValueError:
        hostname = hostname.encode('idna').decode('ascii')
        loopback = hostname == 'localhost'
    if parsed.scheme == 'http' and not loopback:
        raise SystemExit('首次接入 HTTP 面板仅允许回环地址')
    return parsed.scheme, hostname, parsed.port or (443 if parsed.scheme == 'https' else 80)

identity = Path('/etc/sinan/identity')
if identity.exists() or identity.is_symlink():
    protected(identity)
    if not identity.is_dir():
        raise SystemExit('首次接入身份目录必须是普通目录')
    names = {path.name for path in identity.iterdir()}
    if names:
        if ('panel_origin' not in names or not names <= {'panel_origin', 'device.key', 'server_id'}
                or 'server_id' in names and 'device.key' not in names):
            raise SystemExit('已有身份不是可恢复的首次接入状态')
        if origin(ordinary(identity / 'panel_origin', 8192).decode('utf-8').strip()) != origin(sys.argv[1]):
            raise SystemExit('已有身份属于另一面板，请在原面板重新获取令牌')
        if 'device.key' in names:
            key = identity / 'device.key'
            if key.stat().st_mode & 0o077 or len(ordinary(key, 32)) != 32:
                raise SystemExit('首次接入设备密钥必须是私有 32 字节文件')
        if 'server_id' in names:
            value = ordinary(identity / 'server_id', 32).decode('ascii').strip()
            if not re.fullmatch(r'[1-9][0-9]{0,18}', value) or int(value) > 9223372036854775807:
                raise SystemExit('首次接入服务器身份无效')
PY
fi
if [ -e "/opt/sinan/core/$VERSION" ]; then
  [ -d "/opt/sinan/core/$VERSION" ] && [ ! -L "/opt/sinan/core/$VERSION" ] || exit 1
  for file in sinan-agent release.json SHA256SUMS SHA256SUMS.minisig; do
    cmp -- "$STAGE/$VERSION/$file" "/opt/sinan/core/$VERSION/$file"
  done
else
  mv --no-clobber --no-target-directory "$STAGE/$VERSION" "/opt/sinan/core/$VERSION"
  [ ! -d "$STAGE/$VERSION" ] || { echo '版本目录已被并发创建，请重新验证' >&2; exit 1; }
fi
if ! getent group sinan-singbox >/dev/null; then
  if command -v groupadd >/dev/null; then groupadd --system sinan-singbox; else addgroup -S sinan-singbox; fi
fi
if ! getent passwd sinan-singbox >/dev/null; then
  NOLOGIN=$(command -v nologin)
  if command -v useradd >/dev/null; then useradd --system --no-create-home --gid sinan-singbox --shell "$NOLOGIN" sinan-singbox
  else adduser -S -D -H -G sinan-singbox -s "$NOLOGIN" sinan-singbox; fi
fi
install -d -m 0755 -o root -g root /opt/sinan/plugins /opt/sinan/plugins/sing-box /var/lib/sinan
install -d -m 0700 /etc/sinan /etc/sinan/identity /var/lib/sinan/core
install -d -m 2750 -o root -g sinan-singbox /var/lib/sinan/plugins /var/lib/sinan/plugins/sing-box@main /var/lib/sinan/plugins/sing-box@main/revisions
install -d -m 0750 -o sinan-singbox -g sinan-singbox /var/lib/sinan/plugins/sing-box@main/data
CONFIGURATION_CHANGED=1
"/opt/sinan/core/$VERSION/sinan-agent" enroll --panel "$PANEL" --token "$TOKEN"
unset TOKEN
python3 -I - "$VERSION" <<'PY'
#!/usr/bin/env python3
"""Read-only recovery gate for independently signed Agent versions before 0.3.1."""

import errno
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import time
import urllib.parse
import uuid

SEGMENT = re.compile(r"[0-9A-Za-z][0-9A-Za-z.+_-]{0,127}\Z")
LEGACY_NODEQUALITY_VERSION = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2"


def ensure(condition, message):
    if not condition:
        raise ValueError(message)


def protected(path):
    for part in (path,) + tuple(path.parents):
        properties = part.lstat()
        ensure(not part.is_symlink() and properties.st_uid == 0 and properties.st_mode & 0o022 == 0,
               "旧 Agent 状态路径必须由 root 保护，安装未切换")


def regular(path, limit):
    protected(path)
    ensure(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= limit,
           "旧 Agent 状态必须是有界普通文件，安装未切换")
    return path.read_bytes()


def quiescent(saved, allow_missing=False):
    """An old process must not create Preparing after the read-only preflight."""
    environment = os.environ.copy()
    environment["LC_ALL"] = "C"
    try:
        if Path("/run/systemd/system").is_dir():
            result = subprocess.run(["systemctl", "show", "sinan-agent.service", "--property=LoadState",
                                     "--property=ActiveState", "--property=SubState", "--property=MainPID"],
                                    capture_output=True, env=environment, check=False, timeout=3)
            ensure(result.returncode == 0 and len(result.stdout) <= 4096,
                   "无法确认旧 Agent 服务停止，请先执行受控维护，安装未切换")
            properties = {}
            for line in result.stdout.decode("ascii").splitlines():
                key, separator, value = line.partition("=")
                ensure(separator and key not in properties, "旧 Agent 服务状态无效，安装未切换")
                properties[key] = value
            ensure(set(properties) == {"LoadState", "ActiveState", "SubState", "MainPID"}
                   and properties["LoadState"] in (("loaded", "not-found") if allow_missing else ("loaded",))
                   and properties["ActiveState"] == "inactive" and properties["SubState"] == "dead"
                   and properties["MainPID"] == "0", "旧 Agent 仍可能运行，请先执行受控维护，安装未切换")
        elif Path("/run/openrc/softlevel").is_file():
            exists = subprocess.run(["rc-service", "--exists", "sinan-agent"], capture_output=True,
                                    env=environment, check=False, timeout=3)
            status = subprocess.run(["rc-service", "sinan-agent", "status"], capture_output=True,
                                    env=environment, check=False, timeout=3)
            ensure((exists.returncode == 0 and status.returncode == 3)
                   or (allow_missing and exists.returncode == 1 and status.returncode == 1),
                   "无法确认旧 Agent 已停止，请先执行受控维护，安装未切换")
        else:
            raise ValueError("无法确认旧 Agent 服务状态，请先执行受控维护，安装未切换")
    except (OSError, subprocess.TimeoutExpired, UnicodeError):
        raise ValueError("无法确认旧 Agent 服务停止，请先执行受控维护，安装未切换") from None
    endpoint = saved.get("status_socket", "/run/sinan/agent.sock")
    ensure(isinstance(endpoint, str) and Path(endpoint).is_absolute()
           and ".." not in Path(endpoint).parts and not any(ord(character) < 32 for character in endpoint),
           "旧 Agent 状态端点无效，安装未切换")
    connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    connection.settimeout(0.5)
    try:
        connection.connect(endpoint)
    except OSError as error:
        ensure(error.errno in (errno.ENOENT, errno.ECONNREFUSED),
               "无法确认旧 Agent 状态端点停止，安装未切换")
    else:
        raise ValueError("旧 Agent 状态端点仍在运行，请先执行受控维护，安装未切换")
    finally:
        connection.close()


def unique_object(pairs):
    value = {}
    for key, entry in pairs:
        ensure(key not in value, "旧诊断状态字段重复，安装未切换")
        value[key] = entry
    return value


def options_valid(value):
    return (isinstance(value, dict) and len(value) <= 32
            and all(isinstance(key, str) and len(key) <= 128 and isinstance(entry, str)
                    and len(entry) <= 1024 for key, entry in value.items()))


def optional_fields_valid(value):
    return ((value.get("expires_at") is None or type(value["expires_at"]) is int
             and -(2 ** 63) <= value["expires_at"] < 2 ** 63)
            and all(value.get(key) is None or isinstance(value[key], str) and len(value[key]) <= 4096
                    for key in ("start_error", "protection_stop_reason")))


def checkpoint_safe(encoded):
    try:
        checkpoint = json.loads(encoded, object_pairs_hook=unique_object,
                                parse_constant=lambda _value: (_ for _ in ()).throw(ValueError()))
    except (ValueError, RecursionError):
        raise ValueError("旧诊断状态无法安全解析，安装未切换") from None
    if checkpoint is None:
        return
    ensure(isinstance(checkpoint, dict) and len(checkpoint) == 1
           and next(iter(checkpoint)) in ("Preparing", "Started"), "旧诊断检查点未知，安装未切换")
    phase = next(iter(checkpoint))
    value = checkpoint[phase]
    ensure(isinstance(value, dict), "旧诊断检查点损坏，安装未切换")
    if phase == "Started":
        ensure({"spec", "service", "plugin", "started_at"} <= set(value)
               and set(value) <= {"spec", "service", "plugin", "started_at", "start_error",
                                  "expires_at", "protection_stop_reason", "environment"}
               and isinstance(value["spec"], dict) and isinstance(value["service"], dict)
               and value["plugin"] == "nodequality" and type(value["started_at"]) is int
               and 0 <= value["started_at"] < 2 ** 64 and optional_fields_valid(value)
               and (value.get("environment") is None or isinstance(value["environment"], dict)),
               "旧已启动检查点损坏，安装未切换")
        spec, service = value["spec"], value["service"]
        ensure(set(spec) == {"id", "version", "binary_path", "job_dir", "timeout_secs", "options"}
               and {"unit", "program", "args", "working_directory", "timeout_secs"} <= set(service)
               and set(service) <= {"unit", "program", "args", "working_directory", "timeout_secs",
                                    "memory_max", "tasks_max", "cpu_weight", "io_weight", "oom_score_adjust"}
               and isinstance(spec.get("version"), str) and SEGMENT.fullmatch(spec["version"])
               and isinstance(spec.get("id"), str) and len(spec["id"]) == 36
               and isinstance(spec.get("binary_path"), str) and Path(spec["binary_path"]).is_absolute()
               and isinstance(spec.get("job_dir"), str) and Path(spec["job_dir"]).is_absolute()
               and type(spec.get("timeout_secs")) is int and 1 <= spec["timeout_secs"] <= 3600
               and type(service.get("timeout_secs")) is int and service["timeout_secs"] == spec["timeout_secs"]
               and options_valid(spec.get("options")) and isinstance(service.get("args"), list)
               and len(service["args"]) <= 64
               and all(isinstance(argument, str) and len(argument) <= 4096 for argument in service["args"])
               and service.get("working_directory") == spec["job_dir"]
               and service.get("program") == spec["binary_path"]
               and service.get("unit") == "sinan-diagnostic-" + spec["id"] + ".service",
               "旧已启动检查点版本或执行身份无效，安装未切换")
        ensure(all(type(service[key]) is int for key in
                   {"memory_max", "tasks_max", "cpu_weight", "io_weight", "oom_score_adjust"} & set(service)),
               "旧已启动检查点资源形状无效，安装未切换")
        ensure(spec["version"] == LEGACY_NODEQUALITY_VERSION
               and set(spec["options"]) <= {"ip_version", "network_mode", "upload_report"}
               and spec["options"].get("ip_version", "both") in ("both", "ipv4", "ipv6")
               and spec["options"].get("network_mode", "low") in ("low", "normal")
               and spec["options"].get("upload_report", "false") in ("true", "false"),
               "旧 Agent 无法按原版本回收此已启动任务；请保留状态并使用兼容的新签名 Agent，安装未切换")
        try:
            uuid.UUID(spec["id"])
        except (ValueError, TypeError, AttributeError):
            raise ValueError("旧已启动检查点身份无效，安装未切换") from None
        # Existing compiled-root verify-cache validates the exact saved executable proof;
        # the old Agent only observes Started instead of executing it again.
        return
    ensure({"id", "version", "artifact", "timeout_secs"} <= set(value)
           and set(value) <= {"id", "version", "artifact", "timeout_secs", "plugin", "options",
                              "expires_at", "resource_budget"}
           and isinstance(value["version"], str) and SEGMENT.fullmatch(value["version"])
           and type(value["timeout_secs"]) is int and 1 <= value["timeout_secs"] <= 3600
           and isinstance(value["artifact"], dict) and optional_fields_valid(value),
           "旧排队检查点损坏，安装未切换")
    try:
        ensure(isinstance(value["id"], str) and len(value["id"]) == 36, "invalid checkpoint UUID")
        uuid.UUID(value["id"])
    except (ValueError, TypeError, AttributeError):
        raise ValueError("旧排队检查点身份无效，安装未切换") from None
    artifact = value["artifact"]
    proof = artifact.get("proof")
    ensure(set(artifact) == {"url", "sha256", "proof"} and isinstance(artifact["url"], str)
           and 0 < len(artifact["url"]) <= 8192 and isinstance(artifact["sha256"], str)
           and re.fullmatch(r"[0-9a-f]{64}", artifact["sha256"])
           and isinstance(proof, dict) and set(proof) == {"metadata_json", "checksums", "signature"}
           and all(isinstance(proof[key], str) and 0 < len(proof[key]) <= limit
                   for key, limit in (("metadata_json", 32768), ("checksums", 8192), ("signature", 16384))),
           "旧排队检查点签名证明形状无效，安装未切换")
    options = value.get("options", {})
    budget = value.get("resource_budget")
    ensure(budget is None or isinstance(budget, dict)
           and set(budget) == {"memory_max", "tasks_max", "cpu_weight", "io_weight", "oom_score_adjust"}
           and all(type(entry) is int for entry in budget.values()),
           "旧排队检查点资源形状无效，安装未切换")
    ensure(options_valid(options) and value.get("plugin", "nodequality") == "nodequality",
           "旧排队检查点插件或参数未知，安装未切换")
    mode = options.get("mode", "full")
    ensure(mode in ("full", "daily"), "旧排队检查点模式未知，安装未切换")
    ensure(mode != "full", "旧 Agent 存在尚未启动的完整验机任务；恢复门禁拒绝，原状态保留，安装未切换")


def _preflight(version, configuration):
    ensure(isinstance(version, str) and re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?", version),
           "旧 Agent 版本无法安全确认")
    if tuple(int(part) for part in version.split("-")[0].split("+")[0].split(".")) > (0, 3, 0):
        return
    configuration = Path(configuration)
    database, saved = Path("/var/lib/sinan/core/state.db"), {}
    if configuration.exists() or configuration.is_symlink():
        try:
            import tomllib
        except ImportError:
            raise ValueError("旧 Agent 恢复预检需要 Python 3.11 或更新版本，安装未切换") from None
        try:
            saved = tomllib.loads(regular(configuration, 1048576).decode("utf-8"))
        except (ValueError, UnicodeError):
            raise ValueError("旧 Agent 配置无法安全解析，安装未切换") from None
        state_path = saved.get("state_db", str(database))
        ensure(isinstance(state_path, str) and Path(state_path).is_absolute()
               and not any(ord(character) < 32 for character in state_path)
               and ".." not in Path(state_path).parts, "旧 Agent 状态位置无法安全确认，安装未切换")
        database = Path(state_path)
        quiescent(saved, allow_missing=not database.exists())
    if not database.exists() and not database.is_symlink():
        return
    if not (configuration.exists() or configuration.is_symlink()):
        raise ValueError("旧 Agent 状态存在但配置缺失，安装未切换")
    protected(database)
    ensure(database.is_file(), "旧 Agent 状态必须是普通文件，安装未切换")
    wal, shm = (Path(str(database) + suffix) for suffix in ("-wal", "-shm"))
    for sidecar in (wal, shm):
        if sidecar.exists() or sidecar.is_symlink():
            protected(sidecar)
            ensure(sidecar.is_file(), "旧 Agent 状态附属文件无效，安装未切换")
    ensure(not wal.exists() or wal.stat().st_size == 0 or shm.exists(),
           "旧 Agent WAL 缺少共享内存，拒绝创建恢复文件，安装未切换")
    try:
        import sqlite3
    except ImportError:
        raise ValueError("旧 Agent 状态预检需要 Python 标准库 SQLite 支持，安装未切换") from None
    status = lambda: [(path.exists(), path.stat().st_ino, path.stat().st_size, path.stat().st_mtime_ns)
                      if path.exists() else (False,) for path in (database, wal, shm)]
    before, deadline = status(), time.monotonic() + 1
    connection = None
    try:
        uri = "file:" + urllib.parse.quote(str(database), safe="/") + "?mode=ro"
        connection = sqlite3.connect(uri, uri=True, timeout=1)
        connection.set_progress_handler(lambda: int(time.monotonic() >= deadline), 1000)
        rows = connection.execute(
            "SELECT substr(value,1,1048577),typeof(value) FROM kv WHERE key='diagnostics:active' LIMIT 2").fetchall()
    except sqlite3.Error:
        raise ValueError("旧诊断状态无法只读确认，安装未切换") from None
    finally:
        if connection is not None:
            connection.close()
    ensure(status() == before, "旧诊断状态在预检期间变化，安装未切换")
    ensure(len(rows) <= 1, "旧诊断状态重复，安装未切换")
    if rows:
        encoded, kind = rows[0]
        ensure(kind == "text" and isinstance(encoded, str) and len(encoded.encode("utf-8")) <= 1048576,
               "旧诊断状态不是有界 JSON，安装未切换")
        checkpoint_safe(encoded)


def preflight(version, configuration=Path("/etc/sinan/agent.toml")):
    try:
        _preflight(version, configuration)
    except OSError:
        raise ValueError("旧状态路径无法安全读取，安装未切换") from None


if __name__ == "__main__":
    try:
        ensure(len(sys.argv) == 2, "旧 Agent 预检参数无效")
        preflight(sys.argv[1])
    except ValueError as error:
        raise SystemExit(f"Legacy Agent refused: {error}") from None
    except OSError:
        raise SystemExit("Legacy Agent refused: 旧状态路径无法安全读取，安装未切换") from None
PY
ACTIVATING=1
ln -s "$VERSION" "/opt/sinan/core/current.$$"
mv -Tf "/opt/sinan/core/current.$$" /opt/sinan/core/current
ln -sfn /opt/sinan/core/current/sinan-agent /usr/local/bin/sinan-agent
if [ "$INIT" = systemd ]; then
cat > /etc/systemd/system/sinan-agent.service <<'UNIT_AGENT'
[Unit]
Description=Sinan Agent
After=network-online.target
Wants=network-online.target

[Service]
OOMScoreAdjust=-500
CPUWeight=1000
ExecStartPre=/opt/sinan/core/current/sinan-agent verify-installed --binary /opt/sinan/core/current/sinan-agent --name agent --format raw
ExecStart=/opt/sinan/core/current/sinan-agent --config /etc/sinan/agent.toml supervise
Restart=always
RestartPreventExitStatus=78
SuccessExitStatus=78
RestartSec=5
UMask=0027

[Install]
WantedBy=multi-user.target
UNIT_AGENT
cat > /etc/systemd/system/sinan-singbox@.service <<'UNIT_RUNTIME'
[Unit]
Description=Sinan sing-box runtime (%i)
After=network-online.target
Wants=network-online.target
ConditionPathExists=/opt/sinan/plugins/sing-box/current/sing-box
ConditionPathExists=/var/lib/sinan/plugins/sing-box@%i/current/config.json

[Service]
OOMScoreAdjust=-500
CPUWeight=1000
User=sinan-singbox
Group=sinan-singbox
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
NoNewPrivileges=true
ExecStartPre=/usr/bin/test -f /var/lib/sinan/plugins/sing-box@%i/current/config.json
ExecStartPre=/opt/sinan/core/current/sinan-agent verify-installed --binary /opt/sinan/plugins/sing-box/current/sing-box --name sing-box --format tar.gz
ExecStart=/opt/sinan/plugins/sing-box/current/sing-box run -c /var/lib/sinan/plugins/sing-box@%i/current/config.json -D /var/lib/sinan/plugins/sing-box@%i/data
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
UNIT_RUNTIME
systemctl daemon-reload
systemctl enable sinan-singbox@main.service sinan-agent.service
systemctl restart sinan-agent.service
else
cat > /etc/init.d/sinan-agent <<'UNIT_AGENT_OPENRC'
#!/sbin/openrc-run

description="Sinan Agent"
supervisor="supervise-daemon"
command="/opt/sinan/core/current/sinan-agent"
command_args="--config /etc/sinan/agent.toml supervise"
pidfile="/run/${RC_SVCNAME}.pid"
required_files="/etc/sinan/agent.toml"
respawn_delay=5
respawn_max=0
retry="TERM/5/KILL/5"
umask="0027"
output_log="/var/log/sinan/agent.log"
error_log="$output_log"

depend() {
    need net
}

start_pre() {
    /opt/sinan/core/current/sinan-agent verify-installed --binary /opt/sinan/core/current/sinan-agent --name agent --format raw || return 1
    checkpath --directory --mode 0750 --owner root:sinan-singbox /var/log/sinan
    checkpath --file --mode 0640 --owner root:root "$output_log"
}
UNIT_AGENT_OPENRC
cat > /etc/init.d/sinan-singbox@main <<'UNIT_RUNTIME_OPENRC'
#!/sbin/openrc-run

description="Sinan 代理运行时"
supervisor="supervise-daemon"
command="/opt/sinan/plugins/sing-box/current/sing-box"
command_args="run -c /var/lib/sinan/plugins/sing-box@main/current/config.json -D /var/lib/sinan/plugins/sing-box@main/data"
command_user="sinan-singbox:sinan-singbox"
pidfile="/run/${RC_SVCNAME}.pid"
required_files="$command /var/lib/sinan/plugins/sing-box@main/current/config.json"
capabilities="^cap_net_bind_service"
no_new_privs="yes"
respawn_delay=5
respawn_max=0
retry="TERM/5/KILL/5"
umask="0027"
output_log="/var/log/sinan/runtime.log"
error_log="$output_log"
extra_started_commands="reload"
description_reload="重载代理配置"

depend() {
    need net
}

start_pre() {
    /opt/sinan/core/current/sinan-agent verify-installed --binary /opt/sinan/plugins/sing-box/current/sing-box --name sing-box --format tar.gz || return 1
    checkpath --directory --mode 0750 --owner root:sinan-singbox /var/log/sinan
    checkpath --file --mode 0640 --owner sinan-singbox:sinan-singbox "$output_log"
}

reload() {
    ebegin "重载代理配置"
    supervise-daemon "$RC_SVCNAME" --signal HUP
    eend $?
}
UNIT_RUNTIME_OPENRC
chmod 0755 /etc/init.d/sinan-agent /etc/init.d/sinan-singbox@main
rc-update --update
rc-update add sinan-agent default
rc-update add sinan-singbox@main default
rc-service sinan-agent restart
fi
STARTED=0
for attempt in $(seq 1 30); do
  if /opt/sinan/core/current/sinan-agent --config /etc/sinan/agent.toml status >/dev/null 2>&1; then STARTED=1; break; fi
  sleep 1
done
[ "$STARTED" = 1 ] || { echo 'Agent 未通过启动检查' >&2; exit 1; }
COMPLETED=1
printf '%s\n' '已验证并安装 Agent，可运行 sinan-agent status 查看状态。'
SINAN_BOOTSTRAP_C6C95142E68A89AA786F475B3209016DAF80A8320F115DB3215FB7D94B62D439


if [ "$PLATFORM" = Linux ] && ! command -v minisign >/dev/null; then
  MINISIGN=$("$PYTHON" -I -c 'import sys; sys.path.insert(0, sys.argv[1]); import bootstrap; print(bootstrap.prepare_linux_minisign(sys.argv[1]))' "$STAGING")
fi

# Isolated mode ignores Python environment/path overrides; imports use only this bundle.
"$PYTHON" -I -c 'import runpy, sys; sys.path.insert(0, sys.argv.pop(1)); runpy.run_module("bootstrap", run_name="__main__")' "$STAGING" "$@" --trusted-keys "$STAGING/public-keys.json" --minisign "$(command -v "$MINISIGN")"
