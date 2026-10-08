# Native overlay implementation status

## Current result — 2026-09-24 r16

Windows 240 and Android 14 arm64 Redroid have completed actual Rove EasyTier
overlay membership, discovery and bidirectional protocol operations using the
installed r16 packages. This is not a ping, ADB-forward or host-port result.

- Same network: `2ae28437-b51a-4216-b6cb-3bbb48ee25c6` (`VPN 接入测试`).
- Windows Wintun: `10.203.240.3/24`; device
  `8f534803-8bb3-4940-a7e2-c35c50709632`.
- Android VpnService: `10.203.240.2/24`; device
  `e6ba18ba-825f-4135-a99e-118485000963`.
- Both real device lists contain both identities. Targeted `get_device` returns
  200 in both directions with the opposite platform/identity.
- Final Windows GUI PID 14664 starts ordinary-user agent PID 17848; normal
  network startup launched helper PID 26124 with `--agent-pid 17848`, without
  the preauthorized staging pipe. Original identities and saved networks survive
  the installer/APK upgrades. PIDs are observations, not configuration constants.
- Both apps actively connect to public bootstrap `tcp://183.230.36.171:11010`.
  Existing Windows helper inbound block rules remain enabled. No temporary
  acceptance allow rule remains; helper creates the restricted overlay agent
  rule itself. The user's Mihomo proxy/forwarding settings were not changed.
- Stopping Windows produces stopped/empty addresses and zero overlay adapters,
  API listeners and owned overlay allow rules. Android then returns 503
  `target_unreachable`. Restart works. Stopping Android removes tun0 and its
  listeners; Windows returns the same explicit error, never local execution.
- From 237, physical `10.1.2.240:43190` is unavailable. An exact temporary route
  to `10.203.240.3/32` via Windows' physical interface likewise gives HTTP 000 /
  connect timeout; that route was removed in a finally block. Android physical
  address probes are diagnostic only, not a substituted cross-host ingress test.
- Both installed apps obtain eight directory entries from the packaged
  `http://10.1.2.237:8090` default and resolve `txt://et.lctn.cc.cd` to a TCP
  connector. External TXT/HTTPS success is time-dependent, not an uptime promise.
- Native card/layout checks plus 1100×760, 320px, 390px and 844×390 browser
  coverage passed. Remaining coverage: physical Android phones/soft keyboards,
  arbitrary third-party VPN coexistence, and native interoperability of every
  one of the seven accepted connector protocols. These are not claimed complete.

## Shared foundation

`rove-agent::overlay::EasyTier` now supports the existing external RPC backend
and an application-owned upstream `NetworkInstanceManager`. Both use the same
validated JoinConfig conversion, instance identity and original discovery URI.
The native backend explicitly stops owned instances on shutdown; an external
daemon is never terminated by dropping its client.

`easytier-native` enables the upstream TUN and connector implementations. The
pinned upstream Android NIC requires the `magic-dns` compile-time feature;
Rove explicitly disables that runtime setting. This is not permission to alter
system DNS. Native Android descriptors remain owned by Rove until the upstream
instance has stopped, since that upstream NIC uses `close_fd_on_drop(false)`.

Linux kernel validation lives in `overlay_platform/linux.rs`. Windows IP Helper
validation lives in `overlay_platform/windows.rs`; both use the same observer,
device discovery and service lifecycle. Windows requires an active Rove-named
Wintun/EasyTier interface with its actual preferred address, non-overlapping
routes and strong-host interfaces. Connected forwarding interfaces additionally
require authenticated live-helper proof of per-overlay dynamic WFP blocking rules.
It does not change these OS settings. Shared outbound transport binds the verified
source IPv4 (and the interface on Linux/Android), rejects DNS endpoints, disables
proxies and redirects, and does not fall back to the physical network.

Windows native-feature startup is wired to a restricted named-pipe helper;
the r16 installer and normal-user startup passed validation on Windows 240.
The pinned pnet datalink dependency has a Windows-only native interface-enumeration
adaptation in `vendor/pnet_datalink`; this removes the static Npcap import without
duplicating shared network logic. Windows FakeTCP uses upstream WinDivert, whose
signed driver, license and corresponding source archive are included in packaging.
The Npcap fallback explicitly reports Unsupported. Packaging rejects binaries
that still import Packet.dll or wpcap.dll; seven-protocol acceptance is not evidence
that each protocol has passed native end-to-end verification.
Android now has a Kotlin VpnService/consent adapter and JNI descriptor transfer
connected to the shared observer. Android r13 was upgraded in place, retaining
identity; two-network start, independent stop, system disconnect cleanup and
restart were verified on Android 14 Redroid. Installed r16 cross-device packet
exchange and Windows physical-ingress negative boundaries are recorded above.

`overlay::vpn_plan` prepares a complete, validated multi-network routing plan
before changing any native VPN. It rejects overlapping subnets and cross-network
packet injection. `vpn_io` forwards packet-preserving per-instance channels;
`mobile_vpn` replaces the complete native plan, checks consent/service readiness,
and joins old forwarding tasks before admitting a replacement. Native Android
interface names come from TUNGETIFF, not the core's configured alias.

## Integration contract and remaining coverage

- Windows (validated on 240): restricted network helper, account-only/PID-checked
  local IPC, Wintun packaging, dynamic WFP rules and normal-user agent on 240.
  Current-user agent must not be elevated to obtain Wintun privileges. Do not expose an
  unrestricted privileged management listener on a physical interface.
- Android: Redroid lifecycle and inter-device packet exchange passed;
  physical-device/keyboard validation remains. The system provides
  only one active VPN service per user, so a second saved Rove network must not
  silently replace the first. See [Android VPN lifecycle](https://developer.android.com/develop/connectivity/vpn).
  Only overlay routes are installed, never a default route or DNS override.
  The pinned core has no discovered socket-protect hook and skips its ordinary
  per-interface connector binding on Android. Underlay/overlay route coexistence
  therefore remains an explicit native test gate; do not claim arbitrary VPN or
  proxy coexistence from the packet fixtures.
- Both: drive the same observer and peer-discovery logic; do not publish running
  until native interface facts and the isolated Rove listener are verified.
- Delivery: upgrade while retaining device identity and saved configuration,
  then test actual start/stop, shared network identity, distinct device identities,
  overlay Rove operations and physical-network negative boundaries.

## Historical evidence and implementation progression

2026-09-24 r14/r15 native integration (supersedes the earlier missing-runtime findings):

- Staged Windows native binaries and installed Android r14 joined network
  `2ae28437-b51a-4216-b6cb-3bbb48ee25c6`, using actual Wintun/VpnService addresses
  `10.203.240.3` / `10.203.240.2`. Both discovered the other Rove device and
  completed targeted `get_device` in both directions. Device IDs remained
  `8f534803-8bb3-4940-a7e2-c35c50709632` (Windows) and
  `e6ba18ba-825f-4135-a99e-118485000963` (Android). This staging run used scoped
  temporary firewall rules, not yet final installer acceptance.
- Stopping Windows removed the TUN and API listener; the Android target call
  returned `target_unreachable`, not local execution. Physical-IP API access
  failed; a temporary host route from 237 to the Windows overlay IP via its
  physical interface also failed and was removed immediately.
- r15 installer upgrade on 240 returned 0; GUI, CLI, agent, helper and driver
  hashes matched. The ordinary-user GUI started the ordinary-user agent and
  retained the original identity/configuration. Test-only firewall rules were
  removed and the staging rules' prior enabled states restored.
- The helper now installs an agent-image/local-overlay-IP/remote-subnet scoped
  TCP allow rule before API binding. Normal network stop removes that rule.
  Existing explicit block policy is not disabled. Upstream EasyTier also adds
  its own persistent helper application rules; this is not a claim that the
  host firewall is untouched. Forced process termination can leave a stale
  scoped COM allow rule; the vanished helper/TUN is rejected by the observer,
  which closes the remaining listener. Abrupt helper-kill timing was not part
  of the native acceptance run. The WFP isolation rules themselves belong to
  an OS-cleaned dynamic session.
- Installed r15 started Wintun and its API successfully. Windows generated
  helper inbound block rules, so the final test uses outbound public bootstrap
  on both peers, leaving those blocks unchanged. Public relay exposed a shared
  two-second discovery-round budget covering three protocol exchanges; direct
  overlay hello succeeded but background device admission timed out. The shared
  round is now bounded at eight seconds within a fifteen-second lifecycle
  reconciliation; shutdown cancels the wait. Native r16 retest is pending.
- Android r15 retained identity and failed diagnostics, fetched eight public
  nodes, resolved TXT and successfully retried HTTPS → TXT → TCP. The upstream
  TXT service remains intermittently failing (including an independently
  observed DNS SERVFAIL); success is not an uptime guarantee.
- Actual Android and Windows creation flows opened Base64/QR cards. Screenshots
  showed intact QR and fixed copy/back controls; temporary card-test networks
  were deleted without modifying saved user networks. Four viewport browser
  permission/copy/list-heading checks passed. Physical phone/soft keyboard
  testing remains outside the available Redroid environment.
- r16 shared-code regression: agent native-feature library 75/75, SDK isolated
  transport 2/2, GUI unit tests 24/24, full browser suite 92/92, formatting and
  OpenSpec strict validation passed. The first browser run exposed two obsolete
  assertions for the intentionally removed duplicate list heading; after
  updating those assertions the complete suite passed.
- Windows private-directory failure was reproduced with the user's existing
  proxy: direct `10.1.2.237:8090/v1/nodes` returned 200, explicit proxy access
  timed out after five seconds. Only validated local HTTP directories now use
  a direct shared client. Public HTTPS proxy behavior and TLS checks remain.

2026-09-24 Android r12 integration checks:

- Shared overlay tests: 8/8; full agent library serial regression: 68/68.
- Windows native test compilation and Android arm64 APK build passed.
- Four browser viewport permission-error/copy checks passed; screenshots
  inspected. These do not replace native mobile or keyboard validation.
- APK `rove-android-arm64-debug-20260924-r12.apk` installed with `adb install -r`
  and launched; device ID remained `e6ba18ba-825f-4135-a99e-118485000963`.
- An isolated `VPN 接入测试` network was created with local address
  `10.203.240.2/24`, no bootstrap peers. Start opened Android's real
  `com.android.vpndialogs/.ConfirmDialog`; no automated permission approval.
  Awaiting user consent, so actual native start/stop remains unverified.
- No new Windows installer was deployed; no Windows–Android membership proved.

2026-09-24 continued native testing (r12; r13 fixes being built):

- The user clarified that implementation and testing should proceed through
  completion. The normal Android Rove consent dialog was confirmed through the
  test device UI; no app-op override or app privilege escalation was used.
- Redroid lacked `/dev/tun` although `/dev/net/tun` existed. Android's system VPN
  opens `/dev/tun`, so this produced `Cannot allocate TUN: Bad file descriptor`.
  Provisioned only that missing character device (10:200, mode 0660,
  system:vpn) in the test container. Rove remains UID 10087 and uses VpnService;
  this is a test-environment prerequisite, not a root requirement for the app.
- The system then reported `Established by app.rove.desktop on tun0` with
  `10.203.240.2/24`; Rove reported running after its interface/listener checks.
  Explicit stop removed the TUN and Rove VPN foreground service and retained
  network configuration. This does not prove inter-device communication.
- A second nonoverlapping network exposed rejection of Android secondary IPv4
  labels (`tunN:1`). Shared Linux/Android validation now recognizes numeric
  address aliases without weakening interface-name matching. Disabled failed
  networks now retain their diagnostic during cleanup. Full serial regression
  passed 70/70, including both new regressions; updated APK verification pending.
- Android consent retries reuse a pending dialog. IPv6 is allowed to remain on
  the underlying network rather than implicitly blocked by an IPv4-only VPN.

2026-09-24 checks for the transport/platform changes:

- Windows `easytier-native` library/binary and test compilation checks passed.
- Android arm64 `easytier-native` compilation check passed.
- SDK overlay transport regressions: 2/2 passed.
- Agent Linux regression: parallel run 64/65 (migration reopen encountered
  `An agent already owns this data directory`); isolated retry passed and full
  serial retry passed 65/65. The intermittent lock failure is not diagnosed.
- Formatting, diff whitespace and OpenSpec strict validation passed. No new
  installer/APK was generated or deployed; no native membership was verified.

2026-09-24 compatibility finding (read-only): `Get-NetIPInterface` on 240
reported Mihomo interface index 97 connected with forwarding enabled; its
weak-host send/receive flags were disabled. The initial conservative Windows
guard therefore rejects this host. This is not evidence of an actual packet
leak. No proxy, interface, firewall or forwarding setting was changed. Before
delivery, resolve coexistence through a reviewed ingress/egress isolation design
(for example app-owned, scoped WFP filters with lifetime cleanup), not automatic
changes to the user's proxy. Native negative-boundary testing remains required.

2026-09-23 foundation checks:

- Agent library regressions with `--features easytier`: 65 passed.
- Final shared-backend / VPN-plan focused run: 5 passed, including upstream
  running-info/version observation for a real no-TUN instance.
- `cargo check --features easytier-native` passed for
  `aarch64-linux-android` and `x86_64-pc-windows-gnu`.
- OpenSpec strict validation passed. No new Windows installer or Android APK
  was produced or installed for this foundation change.

The shared backend tests start real upstream instances with `no_tun=true` and
no external peers. They verify management and ownership only. Packet-plan tests
do not exercise a native VPN. Compilation checks do not establish membership.
The early stages above did not complete Windows–Android membership. The current
r16 result at the top supersedes that historical limitation only for the tested
Windows 240 / Android Redroid environment; it does not expand the evidence scope.
