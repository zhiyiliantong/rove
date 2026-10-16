# Rove patch to pnet_datalink 0.35.0

Source: crates.io pnet_datalink 0.35.0 (MIT OR Apache-2.0); original licenses
are preserved beside this document. Linux, Android and other non-Windows
implementations are unchanged.

Windows interface enumeration uses netdev's native IP Helper API implementation
(also already used by EasyTier), rather than statically importing Npcap Packet.dll.
The Windows pnet capture fallback explicitly returns Unsupported. EasyTier's
primary Windows x86/x64 FakeTCP backend is WinDivert, not this fallback.
Do not distribute Npcap binaries as part of Rove without the required license.

Patch scope: Cargo.toml, src/lib.rs, src/bindings/mod.rs, and the new
src/windows_native.rs. Keep this adaptation OS-only; do not add Rove business
logic here. Windows packaging must check PE imports for Packet.dll and include
the licensed WinDivert driver for FakeTCP.
