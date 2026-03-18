use core_affinity::CoreId;

#[cfg(target_os = "macos")]
use std::sync::OnceLock;

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThreadQos {
    Background,
    Utility,
    Default,
    UserInitiated,
    UserInteractive,
}

#[cfg(target_os = "macos")]
impl ThreadQos {
    const fn as_raw(self) -> u32 {
        match self {
            Self::Background => 0x09,
            Self::Utility => 0x11,
            Self::Default => 0x15,
            Self::UserInitiated => 0x19,
            Self::UserInteractive => 0x21,
        }
    }

    fn parse(value: &str) -> Result<Option<Self>, ()> {
        let value = value.trim().to_ascii_lowercase();
        match value.as_str() {
            "" | "off" | "none" | "unspecified" => Ok(None),
            "background" => Ok(Some(Self::Background)),
            "utility" => Ok(Some(Self::Utility)),
            "default" => Ok(Some(Self::Default)),
            "user-initiated" | "user_initiated" | "userinitiated" => Ok(Some(Self::UserInitiated)),
            "user-interactive" | "user_interactive" | "userinteractive" => Ok(Some(Self::UserInteractive)),
            _ => Err(()),
        }
    }

    fn current() -> Option<Self> {
        static CONFIGURED_QOS: OnceLock<Option<ThreadQos>> = OnceLock::new();
        *CONFIGURED_QOS.get_or_init(|| match std::env::var("SPACETIMEDB_THREAD_QOS") {
            Ok(value) => match ThreadQos::parse(&value) {
                Ok(qos) => qos,
                Err(()) => {
                    log::warn!(
                        "unrecognized SPACETIMEDB_THREAD_QOS={value:?}; defaulting to user-initiated on macOS"
                    );
                    Some(ThreadQos::UserInitiated)
                }
            },
            Err(_) => Some(ThreadQos::UserInitiated),
        })
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn pthread_set_qos_class_self_np(class: u32, relative_priority: i32) -> i32;
}

/// Apply the current platform's preferred scheduler hint for compute-heavy worker threads.
///
/// On Linux and other non-macOS platforms, this uses CPU affinity when a core is provided.
/// On macOS, affinity is intentionally avoided in favor of a QoS class:
/// `SPACETIMEDB_THREAD_QOS=user-initiated` (default), `utility`, `default`,
/// `background`, `user-interactive`, or `off`.
pub(crate) fn apply_compute_thread_hint(core_id: Option<CoreId>) {
    #[cfg(target_os = "macos")]
    {
        let _ = core_id;
        if let Some(qos) = ThreadQos::current() {
            let rc = unsafe { pthread_set_qos_class_self_np(qos.as_raw(), 0) };
            if rc != 0 {
                log::warn!("pthread_set_qos_class_self_np failed with errno {rc}");
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(core_id) = core_id {
        core_affinity::set_for_current(core_id);
    }
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::ThreadQos;

    #[test]
    fn parses_thread_qos_values() {
        assert_eq!(ThreadQos::parse("user-initiated"), Ok(Some(ThreadQos::UserInitiated)));
        assert_eq!(ThreadQos::parse("utility"), Ok(Some(ThreadQos::Utility)));
        assert_eq!(ThreadQos::parse("off"), Ok(None));
        assert_eq!(ThreadQos::parse("nonsense"), Err(()));
    }
}
