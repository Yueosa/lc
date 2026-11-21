from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
import tomllib


class HealthHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path != "/health":
            self.send_error(404)
            return
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"ok\n")


if __name__ == "__main__":
    config_path = Path(__file__).resolve().parents[1] / "config.toml"
    with config_path.open("rb") as config_file:
        host, port = tomllib.load(config_file)["server"]["listen"].rsplit(":", 1)
    HTTPServer((host, int(port)), HealthHandler).serve_forever()
