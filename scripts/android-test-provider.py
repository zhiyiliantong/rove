"""Local deterministic fixture for Android unsupported-tool acceptance (no API key)."""
import json
from http.server import BaseHTTPRequestHandler, HTTPServer


class Model(BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        finished = any(m.get("role") == "tool" for m in body["messages"])
        delta = {"role": "assistant", "content": "Fixture completed; inspect the actual tool result."} if finished else {
            "role": "assistant", "tool_calls": [{"index": 0, "id": "android_unsupported", "type": "function",
                "function": {"name": "system_exec", "arguments": json.dumps({"command": "echo rove-mobile-test", "timeout_seconds": 5})}}]}
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        for d, reason in [(delta, None), ({}, "stop" if finished else "tool_calls")]:
            chunk = {"id": "fixture", "object": "chat.completion.chunk", "created": 1, "model": "fixture",
                     "choices": [{"index": 0, "delta": d, "finish_reason": reason}]}
            self.wfile.write(("data: " + json.dumps(chunk) + "\n\n").encode())
        self.wfile.write(b"data: [DONE]\n\n")

    def log_message(self, *args):
        pass


HTTPServer(("127.0.0.1", 18411), Model).serve_forever()
