use std::{
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    sync::LazyLock,
};

pub trait Switch {
    fn is_on() -> bool;
}

/// Displays `on` while `S` is on, `off` otherwise.
#[derive(Clone, Copy)]
pub struct Switched<S: Switch, On = &'static str, Off = &'static str> {
    on: On,
    off: Off,
    _switch: PhantomData<S>,
}

impl<S: Switch, On, Off> Switched<S, On, Off> {
    pub const fn new(on: On, off: Off) -> Self {
        Self {
            on,
            off,
            _switch: PhantomData,
        }
    }
}

impl<S: Switch, On: Display, Off: Display> Display for Switched<S, On, Off> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if S::is_on() {
            self.on.fmt(f)
        } else {
            self.off.fmt(f)
        }
    }
}

static TERMINAL_SUPPORTS_EMOJI: LazyLock<bool> =
    LazyLock::new(|| console::Term::stdout().features().wants_emoji());

#[derive(Clone, Copy, Debug)]
pub struct EmojiSwitch;

impl Switch for EmojiSwitch {
    #[inline]
    fn is_on() -> bool {
        mds_config::flags::emojis() && *TERMINAL_SUPPORTS_EMOJI
    }
}

pub type Emoji = Switched<EmojiSwitch>;

pub const SUCCESS_PREFIX: Emoji = Emoji::new("✅ ", "");
pub const DISCOVERED_PREFIX: Emoji = Emoji::new("🔍 ", "");
pub const NETWORK_INTERFACE: Emoji = Emoji::new("🔌 ", "");

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    static ON: AtomicBool = AtomicBool::new(true);

    struct TestSwitch;

    impl Switch for TestSwitch {
        fn is_on() -> bool {
            ON.load(Ordering::Relaxed)
        }
    }

    #[test]
    fn switched_follows_its_switch() {
        const PREFIX: Switched<TestSwitch> = Switched::new("✅ ", "");

        ON.store(true, Ordering::Relaxed);
        assert_eq!(format!("{PREFIX}done"), "✅ done");

        ON.store(false, Ordering::Relaxed);
        assert_eq!(format!("{PREFIX}done"), "done");
    }
}
