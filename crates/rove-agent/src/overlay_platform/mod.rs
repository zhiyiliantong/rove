//! Native boundary only. Network identity, lifecycle and discovery belong to
//! the shared network_runtime module, never to these platform implementations.
#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use linux::*;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
mod windows_isolation;
#[cfg(windows)]
pub(crate) use windows::*;
#[cfg(windows)]
pub(crate) use windows_isolation::IsolationGuard;
#[cfg(all(windows, feature = "easytier-native"))]
mod windows_firewall;
#[cfg(all(windows, feature = "easytier-native"))]
pub(crate) use windows_firewall::{FirewallGuard, verified_agent_image};
