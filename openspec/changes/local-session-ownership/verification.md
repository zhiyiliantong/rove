# Verification — 2026-09-29

## r24 investigation / card follow-up

The user asked to fix intermittent target_unreachable and additionally required cards to regenerate after network edits. Read-only checks on Windows found EasyTier helper TCP connections sourced from Mihomo 198.18.0.1 to the configured bootstrap, alongside an inbound physical connection from Android's host. At 17:36 both actual targeted get_device calls returned 200 without a code or routing change; intermittent recovery is not a permanent fix.

Review of pinned upstream 8428a89 shows IPCollector appends the system-selected local IPv4 to connector candidates even when it was excluded by interface filtering; TCP custom binding races candidate TCP connects and picks the first completed transport before higher-level EasyTier admission. This supports the previous underlay-path competition finding, not proof that every target_unreachable has that cause. Physical-first/excluding third-party VPN changes Rove's egress policy, so user preference was requested; no such policy change has been implemented or applied without confirmation. Tasks 5.2–5.3 remain open.

Cards now invalidate old payload/QR/copied state on save attempts and navigation, regenerate from freshly fetched saved join configuration on every opening, and guard asynchronous completions with a generation counter. Source checks: GUI 30/30, full browser 105/105, strict OpenSpec and 57-operation contracts passed. Native r24 card verification and artifacts are recorded in the delivery section when complete.

r24 delivery completed: both Windows 240 and Android 14 arm64 Redroid were upgraded and launched, loading index-CWpqC_cX.js. Native GUI edits to stopped test networks regenerated both Base64 and QR; decoded Base64 equals the backend latest JoinConfig, and jsQR decoding of the actual native SVG's rasterized modules returns the same Base64. Test networks were deleted; original identity, all 12/18 sessions, models/defaults, networks/states and aliases matched the round's baseline. Four required browser sizes and native screenshots were inspected. Screenshot timing was adjusted to wait for dialog opacity 1, then all five card/edit tests passed again. This is not physical-phone/camera/IME acceptance. See docs/windows-installer-delivery.md r24 for package hashes and boundaries; reconnection tasks remain open pending egress policy confirmation.

## r23 follow-up: network editing and automatic restart

Removed network-device management/global execution-target controls; remote operations start in local conversations. Network editing reuses the creation form, preserves EasyTier identity name, and makes the key read-only with server-side rejection. The user superseded the intermediate manual-stop restriction: saving an active network now warns about brief disconnection and shares a bounded stop/wait/save/start/wait sequence on Windows and Android. Stopped networks remain stopped. Errors distinguish checking, stopping, uncertain save and saved-but-restart-failed outcomes.

GUI units 30/30, production build, full single-worker browser suite 104/104, contract checks and strict OpenSpec validation passed. The browser sequence covers all four required sizes; screenshots were inspected. Initial browser runs failed due to the missing Chrome path and disk-pressure page-load timeouts; the first restart assertion also incorrectly included pre-reload mock calls. Corrected test setup/expectation, then ran the complete suite successfully. Backend network tests 4/4 cover the immutable key and persistent name/bootstrap edit (same r22 backend reused in r23).

Both r23 packages were hash/signature checked, installed over existing Windows 240 / Android 14 arm64 Redroid and launched. Independent native-state polling observed running → stopping → stopped → starting → running after pressing Save and restart; original shared config and local IP were unchanged. Native Tauri invoke is non-writable, so an initial interception-based test could not observe requests and failed; actual-state polling was used instead. Stopped temporary networks exercised name/bootstrap changes, reopen persistence and key-change 400; both temporary networks were then deleted.

Original Windows 12 / Android 17 sessions, identities, model catalogs/defaults, network IDs/start-stop states and local aliases were preserved. Android gained one additional non-test-script session during acceptance; it was left untouched. Windows retains the user's current alias windows. See docs/windows-installer-delivery.md r23 and /mnt/data/rove-deliveries/2026-09-29-r23/ for package hashes and evidence. Both apps remain open. This is Redroid, not physical-phone/real-IME acceptance.

At 17:12 Asia/Shanghai both runtimes were running but bidirectional targeted get_device still returned 503. This does not complete permanent reconnection or this round's Windows–Android grouping acceptance. No system proxy, Mihomo, route or firewall policy was changed; tasks 5.2–5.3 remain open.

## Automated checks

- `cargo test -p rove-agent --lib --no-default-features`: 62 passed.
- `cargo test -p rove-agent --features easytier-native --lib`: 76 passed.
- `cargo test -p rove-protocol --lib`: 7 passed.
- GUI `npm run build`: passed (existing chunk-size advisory remains).
- GUI `npm test`: 24 passed.
- GUI `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=/root/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome npm run test:e2e`: 96 passed.
- `openspec-cn validate local-session-ownership --strict`: passed.

## Behavioral evidence

- New GUI sessions omit both request target and body execution_target after selecting either the local device or a peer. New model catalog requests are local; old linked conversations retain their original execution/model target.
- Failed creation remains visible across background refreshes. Copy returns the actual error; explicit retry keeps the same session ID/body and opens the created conversation.
- The management tool integration test uses two real agents and the SDK HTTP protocol on test-only loopback routes, with a deterministic model fixture on the peer. It checks peer identity, remote session creation, idempotent create/submit, actual peer run completion and returned output, unchanged origin conversation/history, and no local settings mutation after peer shutdown.
- Provider tool schemas inline SocketTarget; remote session operations require an explicit other device, stable creation ID, and no nested execution_target. Production transport reuses the discovered overlay client.

## Visual inspection

Inspected error and successful creation screenshots at 1100×760, 320×640, 390×844, and 844×390 under `apps/rove-gui/test-results/local-session-*/`. Error text wraps; copy and global add remain reachable; mobile list/detail navigation and the composer remain visible. Short screens scroll the suggestion content independently. Browser tests exercised copy, retry, back navigation and checked horizontal overflow. Existing network/QR responsive tests also passed.

## Boundaries

The checks above are source, host-agent integration, and browser responsive results. Loopback SDK tests alone are not Windows–Android EasyTier grouping evidence. No user session/database migration or deletion was performed.

## Installed r17 acceptance (2026-09-29 follow-up)

Android arm64 Debug APK and Windows x86_64 installer were built, signature/hash checked, installed over r16, and launched on Android 14 Redroid and Windows 240. Native GUI creation after selecting self/peer produced local sessions without execution_target, including when the peer was temporarily unavailable. Identity, all original sessions, model catalog/default and original network start/stop states were compared before/after and preserved.

Actual Android-origin rove_api calls traversed the application EasyTier overlay to Windows; the peer executed a harmless echo with exit code 0 and returned real stdout/tool events to the unchanged Android-owned conversation. A fresh origin conversation after stopping the Windows network received target_unreachable with no local system_exec. The physical Windows API endpoint probe failed. Temporary model connections were removed; auxiliary test sessions were archived (recoverable), and successful test history was retained. No paid provider was called.

After the stop-negative test, both networks returned to running, but final checks at 10:31 Asia/Shanghai still returned target_unreachable in both directions. Reconnection recovery has not passed; restoring start/stop state is not evidence of restored communication. The failure snapshots are retained as recovery-android/recovery-windows JSON.

See `docs/windows-installer-delivery.md` r17 for hashes, run IDs, evidence directory, delayed discovery and the localhost test-provider workaround. This does not claim physical-phone/keyboard/camera acceptance or all connector protocols were retested. Model generation was deterministic; protocol and OS execution were real.

## 2026-09-29 follow-up: reconnection and Chinese command output

At 13:54 Asia/Shanghai the unchanged r17 peers were online again and both targeted get_device calls returned 200. Two Windows stop/start rounds recovered bidirectional calls at 7.044s and 6.379s, an Android round at 6.724s, and a Windows round held stopped for 45s recovered at 9.481s (sampled upper bounds, not exact transition times). Each stopped-network negative returned 503. No bootstrap, proxy, firewall or user network configuration was changed for these checks. This does not identify or fix the earlier prolonged offline episode; tasks 5.1–5.3 remain open.

The installed r17 Windows agent reproduced the reported Chinese ping/stdout/stderr corruption in native tool_output and tool_finished records, not only DOM rendering. The fix replaces unconditional UTF-8 loss with shared bounded framing and a thin Windows code-page conversion, preserving valid UTF-8 and existing Unix handling. No old history is rewritten.

Checks: host agent 64/64 without easytier-native and 78/78 with it; extracted production decoder tests on Windows 240 3/3. Test model generation is a loopback-only deterministic fixture; it does not call a paid provider. Installed fix validation is recorded in the r18 delivery section when complete.

r18 was installed and the actual native conversation now preserves Chinese ping, stdout, stderr, tool events and persisted history (successful run `85d3608c-f16b-477c-ac6d-c239bd97a893`). All three test-only model connections were removed and before/after model catalogs, identity and original sessions matched. See the delivery record for the currently loaded diagnostic-agent hash and exact package distinction.

Reconnection was reproduced again. Opt-in bounded allowlisted EasyTier trace was added and its sanitization test passed. With system routing the Windows runtime selected Mihomo and failed the 38-second recovery round; a temporary helper-only interface exclusion yielded three passing rounds at 8.524/6.432/4.078s with real direct-peer routes and bidirectional get_device. All diagnostic firewall rules were removed; both directions still returned 200 at 14:32. The test identifies an underlay-path issue but is not a shipped permanent policy. The default underlay/VPN strategy requires user direction; tasks 5.2–5.3 remain open.
# Local device aliases — 2026-09-29

- 本机私有 metadata 地址簿，network_id/device_id 绑定；去首尾空白、大小写不敏感的本机全局去重；1–64 字符，不含控制字符，最多 512 条；null 清除。无数据库版本迁移，不改对端 display_name/hello/网络分享。
- list_device_aliases、set_device_alias、resolve_device_alias 使用共享协议；socket 可用，任何 target 被拒绝，overlay HTTP 返回 local_only。会话工具仅暴露读取/解析，不自主修改别名；模型提示要求先本地解析名称，再按稳定目标调用，不按 IP 猜测或在离线时本机替代执行。
- 网络详情点击设备名称直接原位编辑，回车/失焦保存、Esc 取消、输入法组合键不提交，无独立修改表单；支持重名提示、留空清除。网络详情按钮称“管理此设备”；同步配置弹框仍称“选择设备”，不改变新会话本机归属。
- Rust agent 无 native feature 66/66、easytier-native 81/81、协议 7/7；包括持久化重启、本机隔离、重名拒绝、非法输入、清除、HTTP 隐私边界、当前地址和目标隔离。GUI 单元测试 24/24，类型及生产构建通过。
- 浏览器全量初跑 98/100；两项同步配置测试发现复用组件按钮文案影响，修复后这两项与四种尺寸别名测试一起复测 6/6。没有把初跑记录改成 100/100。
- 1100×760、320×640、390×844、844×390 实际截图已查看；修复 390px 长别名被管理按钮挤窄的问题。覆盖保存后刷新、跨网络重名拒绝、长名称换行、清除、选择管理目标后本机新建会话。
- 截图位于 apps/rove-gui/test-results/device-alias-local-device-aliases-<尺寸>/{alias,duplicate}.png。本轮是源码与开发机浏览器/共享 agent 测试，未重新打包或覆盖安装 Windows/Android，未做新别名功能的实机 VPN/软键盘或模型驱动远端安装验收。原任务 5.2–5.3 仍待处理。

## r21 安装版与原位编辑（后续完成）

上述未打包状态已由本节更新：Windows/Android r21 均已重新打包、哈希/签名核对并覆盖安装启动。完整浏览器回归 100/100，原位编辑专项四尺寸和同步配置 6/6，截图已实际检查；初次全量缺少 Chrome 可执行路径导致启动失败，指定本机路径后重跑通过。

Windows 保留用户 `aa`，原位聚焦/取消与别名解析/重名拒绝验证通过；Android 验证原位保存、取消、IME 组合键保护，触摸前台聚焦后外部点击保存和页面重新进入持久化。未聚焦 WebView 的初次程序化 blur 不触发事件，不作为成功证据。两端共享最新前端 index-D8sClFN4.js，实际截图分别 1100×760 和 360×568，无横向溢出。Android 临时别名清除，Windows `aa` 未改动；原身份、会话、模型/连接/默认型号、网络 ID/启停状态与基线一致。

交付制品、安装器脚本首次枚举 agent 的非零状态及后续真实启动确认、数据保留和限制详见 docs/windows-installer-delivery.md r21。证据目录 /mnt/data/rove-deliveries/2026-09-29-r21/。本次没有实体手机/真实中文软键盘或模型按别名远端部署验收。两端网络 running 但 15:43 双向 get_device 503，只发现自身；不能宣称本轮组网成功，原永久重连任务 5.2–5.3 未完成。
