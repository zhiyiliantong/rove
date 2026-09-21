"""Serve an explicit Rove acceptance bundle, never an agent API or directory listing.

Only three fixed packages, calculated checksums and two optional screenshots
are public. Defaults to loopback; LAN binding must be explicitly requested.
No database, logs, source tree, OAuth state or debug ports are exposed.
"""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import html
from pathlib import Path
import shutil
from urllib.parse import urlsplit

PACKAGES = {
    "Rove_0.1.0_amd64.deb": "Ubuntu 24.04 · 桌面界面（amd64）",
    "rove-runtime_0.1.0_amd64.deb": "Ubuntu 24.04 · CLI / agent / EasyTier（amd64）",
    "rove-android-arm64-debug.apk": "Android · arm64 开发测试包",
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=4174)
    parser.add_argument("--linux-screenshot", type=Path)
    parser.add_argument("--android-screenshot", type=Path)
    args = parser.parse_args()
    root = args.directory.resolve(strict=True)
    files = {}
    checksums = []
    cards = []
    for name, label in PACKAGES.items():
        path = (root / name).resolve(strict=True)
        if path.parent != root or not path.is_file():
            raise ValueError("Package must be a regular file in the selected directory")
        with path.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        files["/" + name] = (path, "application/octet-stream", True)
        checksums.append(f"{digest}  {name}\n")
        cards.append(f'<a class="download" href="/{name}" download><strong>{label}</strong>'
                     f'<span>{path.stat().st_size / 1024 / 1024:.1f} MiB · 下载</span></a>')
    screenshots = []
    for kind, path, label in [("linux", args.linux_screenshot, "Linux 原生 WebKit 窗口"),
                              ("android", args.android_screenshot, "Android 实际 WebView")]:
        if path:
            path = path.resolve(strict=True)
            if not path.is_file() or path.suffix.lower() != ".png":
                raise ValueError("Screenshot must be a PNG file")
            files[f"/{kind}.png"] = (path, "image/png", False)
            screenshots.append(f'<figure><a href="/{kind}.png"><img src="/{kind}.png" alt="{label}" loading="lazy"></a><figcaption>{label}</figcaption></figure>')
    page = f'''<!doctype html><html lang="zh-CN"><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>Rove · 核心功能验收</title>
<style>
:root{{font-family:system-ui,sans-serif;color:#283c35;background:#f6f8f5}}*{{box-sizing:border-box}}
body{{max-width:1040px;margin:auto;padding:36px 24px;line-height:1.8}}h1{{font-size:clamp(2rem,5vw,3.2rem);margin:0}}
.eyebrow{{color:#27664f}}.muted{{color:#62796b}}.panel,figure{{background:white;border:1px solid #dce5dd;border-radius:16px;padding:24px;margin:20px 0}}
.downloads{{display:grid;gap:12px}}a{{color:#27664f}}.download{{display:flex;justify-content:space-between;gap:16px;padding:16px;border:1px solid #cbded1;border-radius:12px;text-decoration:none;flex-wrap:wrap}}
.download:hover{{background:#eaf1e8}}.download span{{white-space:nowrap}}code{{overflow-wrap:anywhere}}pre{{overflow:auto;background:#f0f5ef;padding:16px;border-radius:10px}}
.screens{{display:grid;grid-template-columns:2fr 1fr;gap:18px}}figure{{padding:12px}}img{{width:100%;height:auto;border-radius:10px}}figcaption{{font-size:.85rem;color:#62796b}}.boundary{{border-left:4px solid #c49030}}
@media(max-width:650px){{body{{padding:24px 16px}}.screens{{grid-template-columns:1fr}}.panel{{padding:18px}}}}
</style>
<p class="eyebrow">ROVE · {html.escape(root.name)} 开发验收包</p>
<h1>数据自己掌握，跳出平台控制</h1><p>核心功能已接入真实 agent。请安装应用验收；本页只是下载与验收说明，不是浏览器版 Rove。</p>
<section class="panel"><h2>下载安装包</h2><div class="downloads">{''.join(cards)}</div><p><a href="/SHA256SUMS" download>SHA-256 校验文件</a> · Debug / dev 构建，不是正式签名发行版。</p></section>
<section class="panel"><h2>建议验收顺序</h2><ol>
<li>切换中文 / English，检查桌面侧栏、移动底栏及页面适配。</li>
<li>添加自己的 API 连接，选择多个型号、修改名称和默认型号；账号会员登录尚未开放。</li>
<li>创建会话，检查流式气泡、富文本、复制；归档后到归档管理恢复或删除。</li>
<li>在 Ubuntu 创建或加入网络，检查自动地址 / 手动网段、多个初始节点；有另一台已联网 Linux 设备时验证 API 配置同步。</li>
<li>Ubuntu 关闭窗口后从 CLI 查看作业，再打开窗口确认历史；服务通过对话管理，服务页仅查询和访问。</li>
</ol><p class="muted">测试中的可控模型只验证调用与生命周期，不代表真实供应商的智能决策。你的 API 配置只输入安装后的 Rove，不要提交到此下载页。</p></section>
<section class="panel"><h2>Ubuntu 安装</h2><p>桌面安装两个包；无界面设备只需 runtime 包。首次启用服务：</p>
<pre>sudo apt install ./rove-runtime_0.1.0_amd64.deb ./Rove_0.1.0_amd64.deb
sudo systemctl daemon-reload
sudo systemctl enable --now rove-easytier.service
systemctl --user daemon-reload
systemctl --user enable --now rove-agent.service
rove status</pre><p>已有安装先停止 agent 并备份数据，再更新。仅为一个 OS 账号启用 agent；不要以管理员身份进行日常 AI 操作。</p>
<h2>Android 升级</h2><pre>adb -s &lt;设备序列号&gt; install -r rove-android-arm64-debug.apk</pre><p>保留应用数据覆盖安装；签名不一致时不要通过卸载绕过。10.1.2.242 测试环境可通过已有 SSH 隧道连接 ADB，再用 scrcpy 查看。</p></section>
<section class="panel boundary"><h2>尚未交付的范围</h2><p>官方账号登录与会话适配、Windows overlay / 安装器、Android native VPN 与后台常驻、物理相机扫码验收。macOS / iOS 仍暂缓。</p><p>因此本包不代表 Android → Windows 跨网络控制已经完成，也不代表全部 OpenSpec 任务完成。</p></section>
<div class="screens">{''.join(screenshots)}</div>
<p class="muted">此入口仅提供指定制品与截图，没有开放 agent、ADB、数据库或工作目录。</p></html>'''.encode()
    checksum_bytes = "".join(checksums).encode()

    class Handler(BaseHTTPRequestHandler):
        def respond(self, head=False):
            path = urlsplit(self.path).path
            data = page if path in ["/", "/index.html"] else checksum_bytes if path == "/SHA256SUMS" else None
            if data is not None:
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8" if path != "/SHA256SUMS" else "text/plain; charset=utf-8")
                self.headers_common(len(data))
                if not head:
                    self.wfile.write(data)
                return
            if path not in files:
                self.send_error(404)
                return
            file, mime, download = files[path]
            with file.open("rb") as source:
                self.send_response(200)
                self.send_header("Content-Type", mime)
                if download:
                    self.send_header("Content-Disposition", f'attachment; filename="{file.name}"')
                self.headers_common(file.stat().st_size)
                if not head:
                    shutil.copyfileobj(source, self.wfile, 64 * 1024)

        def headers_common(self, size):
            self.send_header("Content-Length", str(size))
            self.send_header("X-Content-Type-Options", "nosniff")
            self.send_header("Cache-Control", "no-store")
            self.send_header("Content-Security-Policy", "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; frame-ancestors 'none'")
            self.end_headers()

        def do_GET(self):
            self.respond()

        def do_HEAD(self):
            self.respond(head=True)

    server = ThreadingHTTPServer((args.host, args.port), Handler)
    print(f"Acceptance downloads: http://{args.host}:{args.port}/ (no agent API)", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
