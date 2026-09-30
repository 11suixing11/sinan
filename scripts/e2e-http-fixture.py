#!/usr/bin/env python3
"""Serve bounded, deterministic HTTP traffic inside the acceptance network."""

import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


DOWNLOAD_BYTES = 2 * 1024 * 1024
MAX_UPLOAD_BYTES = 2 * 1024 * 1024


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        if self.path != "/download":
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(DOWNLOAD_BYTES))
        self.end_headers()
        chunk = b"s" * 65536
        for _ in range(DOWNLOAD_BYTES // len(chunk)):
            self.wfile.write(chunk)

    def do_POST(self):
        if self.path != "/upload":
            self.send_error(404)
            return
        try:
            length = int(self.headers.get("Content-Length", "-1"))
        except ValueError:
            self.send_error(400)
            return
        if not 0 < length <= MAX_UPLOAD_BYTES:
            self.send_error(413)
            return
        remaining = length
        while remaining:
            chunk = self.rfile.read(min(remaining, 65536))
            if not chunk:
                self.close_connection = True
                return
            remaining -= len(chunk)
        payload = (str(length) + "\n").encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/plain")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, _format, *_args):
        pass


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="仅用于隔离验收网络的双向流量夹具")
    parser.add_argument("--listen", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=18081)
    arguments = parser.parse_args()
    ThreadingHTTPServer((arguments.listen, arguments.port), Handler).serve_forever()
