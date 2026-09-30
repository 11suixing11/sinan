#!/usr/bin/env python3
"""Capture and render bounded upstream reports without extracting ZIP paths."""

import base64
import io
import json
import os
import pathlib
import re
import sys
import zipfile


MAX_CAPTURE = 12 * 1024 * 1024
MAX_UNPACKED = 32 * 1024 * 1024
MAX_TEXT = 256 * 1024
SECTIONS = (
    ("header_info", "报告信息"),
    ("hardware_quality", "硬件质量"),
    ("ip_quality", "IP 质量"),
    ("net_quality", "网络质量"),
    ("backroute_trace", "回程路由"),
)
ALLOWED = {
    "header_info.log", "hardware_quality.log", "hardware_quality.json",
    "ip_quality.log", "ip_quality.json", "net_quality.log", "net_quality.json",
    "backroute_trace.log", "backroute_trace.json", "port.log", "yabs.json",
    "basic_info.log",
}


def write_atomic(path, data):
    temporary = path.with_name(path.name + ".tmp")
    with temporary.open("wb") as target:
        target.write(data)
        target.flush()
        os.fsync(target.fileno())
    temporary.chmod(0o600)
    temporary.replace(path)


def clean_text(text):
    text = re.sub(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)", "", text)
    text = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", text)
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    return "".join(c for c in text if c in "\n\t" or ord(c) >= 32 and ord(c) != 127)


def capture(root):
    data = sys.stdin.buffer.read(MAX_CAPTURE + 1)
    if len(data) > MAX_CAPTURE:
        raise ValueError("report upload exceeds its size limit")
    write_atomic(root / "upload.base64", data)


def stream_log(path):
    tail = b""
    while True:
        block = sys.stdin.buffer.read1(8192)
        if not block:
            break
        tail = (tail + block)[-MAX_TEXT:]
        write_atomic(path, tail)
    if not tail:
        write_atomic(path, b"")


def capture_response(root):
    first = b""
    tail = b""
    while True:
        block = sys.stdin.buffer.read1(8192)
        if not block:
            break
        first = (first + block)[:65536]
        tail = (tail + block)[-64:]
    status = re.search(rb"\nSINAN_RESPONSE_STATUS:(\d{3})$", tail)
    if status and first.endswith(status[0]):
        first = first[:-len(status[0])]
    write_atomic(root / "upload-response.txt", first)
    write_atomic(root / "upload-status.txt", status[1] if status else b"")
    sys.stdout.buffer.write(first)


def validate_json(data):
    text = data.decode("utf-8")
    decoder = json.JSONDecoder()
    count = 0
    while text.strip():
        value, end = decoder.raw_decode(text.lstrip())
        if not isinstance(value, dict) or not value:
            raise ValueError("upstream report JSON must contain objects")
        count += 1
        text = text.lstrip()[end:]
    if not count:
        raise ValueError("upstream report JSON is empty")


def render(root):
    capture_path = root / "upload.base64"
    with capture_path.open("rb") as source:
        encoded = source.read(MAX_CAPTURE + 1)
    if len(encoded) > MAX_CAPTURE:
        raise ValueError("report upload exceeds its size limit")
    archive_bytes = base64.b64decode(b"".join(encoded.split()), validate=True)
    files = {}
    total = 0
    with zipfile.ZipFile(io.BytesIO(archive_bytes)) as archive:
        records = archive.infolist()
        if len(records) > len(ALLOWED):
            raise ValueError("report archive contains too many entries")
        for record in records:
            if record.filename not in ALLOWED or record.filename in files or record.is_dir():
                raise ValueError("report archive contains an unexpected or duplicate path")
            total += record.file_size
            if record.file_size > 8 * 1024 * 1024 or total > MAX_UNPACKED:
                raise ValueError("report archive exceeds its uncompressed size limit")
            files[record.filename] = archive.read(record)
    write_atomic(root / "report.zip", archive_bytes)
    for name, _ in SECTIONS:
        log = files.get(name + ".log", b"")
        if not clean_text(log.decode("utf-8", errors="replace")).strip():
            raise ValueError("incomplete upstream report: " + name)
        if name != "header_info":
            validate_json(files.get(name + ".json", b""))
    parts = ["NodeQuality 节点报告\n"]
    for name, title in SECTIONS:
        log = clean_text(files[name + ".log"].decode("utf-8", errors="replace")).strip()
        parts.append("\n===== " + title + " =====\n" + log + "\n")
    response_path = root / "upload-response.txt"
    response = response_path.read_bytes()[:65536].decode("utf-8", errors="replace") if response_path.exists() else ""
    status_path = root / "upload-status.txt"
    status = status_path.read_text().strip() if status_path.exists() else ""
    match = re.search(r"https://nodequality\.com/r/([A-Za-z0-9_-]{1,128})(?=$|\s|[\"'<>])", response)
    if (root / "upload-disabled.txt").exists():
        parts.append("\n公开报告上传已关闭，本地报告已保留。\n")
    elif status.isdigit() and 200 <= int(status) < 300 and match:
        report_url = match.group(0)
        write_atomic(root / "report-url.txt", (report_url + "\n").encode())
        parts.append("\n在线报告：" + report_url + "\n")
    else:
        parts.append("\n在线报告上传未成功，本地报告已保留。\n")
        if status:
            parts.append("HTTP 状态：" + status + "\n")
        if response.strip():
            parts.append(clean_text(response[:4096]).strip() + "\n")
    encoded_text = "".join(parts).encode("utf-8")
    if len(encoded_text) > MAX_TEXT:
        suffix = "\n报告文本已截断，完整原始结果保存在本地 report.zip。\n".encode()
        encoded_text = encoded_text[:MAX_TEXT - len(suffix)].decode("utf-8", errors="ignore").encode() + suffix
    write_atomic(root / "result.txt", encoded_text)
    capture_path.unlink()


def main():
    mode, target = sys.argv[1:]
    root = pathlib.Path(target)
    if mode == "capture":
        capture(root)
    elif mode == "stream-log":
        stream_log(root)
    elif mode == "render":
        render(root)
    elif mode == "response":
        capture_response(root)
    else:
        raise ValueError("unknown report operation")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, zipfile.BadZipFile, RuntimeError) as error:
        print("NodeQuality report collection failed: " + str(error), file=sys.stderr)
        raise SystemExit(1)
