/// How a provider's agent accepts a native interrupt.
///
/// The Worker owns the delivery mechanism under the same authority,
/// reservation and receipt fences for either variant; this is provider policy
/// data, not a capability claim. There is no default delivery: a provider
/// that does not declare one stays [`InterruptDelivery::Unsupported`], so a
/// new provider can never inherit an unverified SIGINT.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InterruptDelivery {
    /// No delivery is declared; an interrupt is refused as not delivered.
    #[default]
    Unsupported,
    /// SIGINT to the claimed agent process. Only providers that install a
    /// SIGINT handler that cancels the turn may declare this.
    Signal,
    /// Write the provider's interrupt key to the terminal. TUIs that read
    /// Ctrl-C/Esc as a key (rather than a signal) cancel this way; a process
    /// SIGINT would terminate them instead.
    Key(&'static [u8]),
}

impl InterruptDelivery {
    pub fn key(self) -> Option<&'static [u8]> {
        match self {
            Self::Key(key) => Some(key),
            Self::Signal | Self::Unsupported => None,
        }
    }

    pub fn unsupported(self) -> bool {
        matches!(self, Self::Unsupported)
    }
}

#[cfg(test)]
#[path = "../unit_tests/interrupt.rs"]
mod tests;
