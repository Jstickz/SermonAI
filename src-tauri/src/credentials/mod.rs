//! The one place credentials come from (PRD §17, Phase 2 of
//! `docs/SermonAI_API_Key_Strategy_Prompt.md`).
//!
//! **No service client reads an environment variable or a config file for a
//! key.** They ask this module, which is what makes the gateway swappable: when
//! Phase 3 lands, a new [`Provider`] replaces the development one and no
//! streaming, summarisation or scripture code changes.
//!
//! ## The three modes, and why the table is not uniform
//!
//! | Service | Managed | BYOK |
//! |---|---|---|
//! | Deepgram | yes | yes |
//! | Anthropic | yes | yes |
//! | YouVersion | yes | **no** |
//! | Tyndale NLT | yes | **no** |
//!
//! YouVersion and Tyndale are managed-only because the licence is *SermonAI's*,
//! not the church's: our app registration carries accepted publisher terms for
//! the versions we ship, and a church's own key would authenticate fine and
//! return nothing it is allowed to display. Offering BYOK there would look like
//! a feature and behave like a bug.
//!
//! ## What a client gets back
//!
//! An [`Access`], never a bare string. Managed mode does not have a vendor key
//! to hand out — the gateway holds it — so a client that expected `String`
//! would have to be rewritten later. Asking a client to carry that shape now,
//! while only the development provider exists, is the whole point of the phase.

pub mod dev;

use std::fmt;

use crate::error::{Error, Result};

/// A service SermonAI authenticates to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Service {
    Deepgram,
    Anthropic,
    YouVersion,
    TyndaleNlt,
}

impl Service {
    /// Whether a church may supply its own key for this service.
    pub fn allows_byok(self) -> bool {
        match self {
            Service::Deepgram | Service::Anthropic => true,
            // The licence is ours, not theirs. See the module note.
            Service::YouVersion | Service::TyndaleNlt => false,
        }
    }

    /// The environment variable the development provider reads.
    pub fn dev_env_var(self) -> &'static str {
        match self {
            Service::Deepgram => "DEEPGRAM_API_KEY",
            Service::Anthropic => "ANTHROPIC_API_KEY",
            Service::YouVersion => "YVP_APP_KEY",
            Service::TyndaleNlt => "TYNDALE_API_KEY",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Service::Deepgram => "Deepgram",
            Service::Anthropic => "Anthropic",
            Service::YouVersion => "YouVersion",
            Service::TyndaleNlt => "Tyndale NLT",
        }
    }
}

/// How a client should reach a service.
///
/// Deliberately not a `String`. In managed mode there is no vendor key on this
/// machine to return — the gateway holds it and the app proves itself with an
/// install token — so a client written against a bare key would break when the
/// gateway lands. Matching on this instead is the change that will not need
/// making twice.
///
/// `Debug` is safe to derive here only because every credential inside is a
/// [`Secret`], which masks itself. Adding a bare `String` field would silently
/// undo that.
#[derive(Debug)]
pub enum Access {
    /// A vendor key held on this machine: a development `.env`, or a church's
    /// own key from the OS keychain.
    DirectKey(Secret),
    /// Route through the SermonAI Gateway, proving identity with the install
    /// token. Not issued by any provider yet — Phase 3.
    Gateway {
        base_url: String,
        install_token: Secret,
    },
}

/// A credential that will not leak into a log or an error message.
///
/// `Debug` and `Display` both print a mask. Rust makes this worth doing: a
/// `#[derive(Debug)]` anywhere up the call chain, or a `tracing` field, would
/// otherwise print the key in full, and the one place that happens is usually
/// an error path nobody tested.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The real value. Call this at the point of use — building a request
    /// header — and never store the result.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Last four characters, for a settings screen that shows which key is in
    /// use without showing the key.
    pub fn masked(&self) -> String {
        let visible: String = self
            .0
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        // A key too short to mask is almost certainly a paste error, and
        // showing four of six characters would be worse than showing none.
        if self.0.chars().count() <= 8 {
            "••••".to_string()
        } else {
            format!("••••{visible}")
        }
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secret({})", self.masked())
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.masked())
    }
}

/// What the operator should be told about a service (Phase 2, item 4).
///
/// Mirrored by `CredentialStatus` in `src/lib/types.ts`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CredentialStatus {
    /// Reached through the gateway on SermonAI's account.
    ManagedActive,
    /// Using a key the church supplied. `masked` is safe to display.
    ByokActive { masked: String },
    /// No install token and no key: onboarding has not finished.
    NotActivated,
    /// The gateway recognised the token and refused it.
    TokenRevoked,
    /// Within the plan but out of allowance for the period.
    QuotaReached,
    /// The gateway could not be reached. Distinct from revoked on purpose: one
    /// is a billing conversation and the other is a network cable, and telling
    /// a church the wrong one wastes a Sunday.
    GatewayUnreachable,
    /// Development only: the service has no key in `.env`.
    DevKeyMissing { env_var: String },
}

impl CredentialStatus {
    /// Whether a client can make a call right now.
    pub fn is_usable(&self) -> bool {
        matches!(
            self,
            CredentialStatus::ManagedActive | CredentialStatus::ByokActive { .. }
        )
    }
}

/// Where credentials come from. One implementation today, two after Phase 3.
pub trait Provider: Send + Sync + 'static {
    fn access(&self, service: Service) -> Result<Access>;
    fn status(&self, service: Service) -> CredentialStatus;
}

/// The process-wide provider, held in `AppState`.
pub struct Credentials {
    provider: Box<dyn Provider>,
}

impl Credentials {
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self { provider }
    }

    /// How to reach a service, or why you cannot.
    pub fn access(&self, service: Service) -> Result<Access> {
        self.provider.access(service)
    }

    pub fn status(&self, service: Service) -> CredentialStatus {
        self.provider.status(service)
    }

    /// Reject a BYOK key for a service that cannot accept one.
    ///
    /// Lives here rather than in the settings UI so the rule holds whatever
    /// calls it — a future importer or a command-line flag included.
    pub fn check_byok_allowed(service: Service) -> Result<()> {
        if service.allows_byok() {
            return Ok(());
        }
        Err(Error::Config(format!(
            "{} cannot use a church's own key: the licence is SermonAI's, and your key would not carry it.",
            service.label()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_never_prints_itself() {
        let secret = Secret::new("dg_live_abcdefghijklmnop");

        // The formatting traps: both must mask, because either can be reached
        // by a derive or a tracing field someone adds later without thinking
        // about it.
        assert_eq!(format!("{secret}"), "••••mnop");
        assert_eq!(format!("{secret:?}"), "Secret(••••mnop)");
        assert!(!format!("{secret:?}").contains("abcdefgh"));

        // And the value is still reachable where it is actually needed.
        assert_eq!(secret.expose(), "dg_live_abcdefghijklmnop");
    }

    #[test]
    fn a_short_secret_shows_nothing_rather_than_most_of_itself() {
        // Four of six characters is not a mask. A key this short is a paste
        // error anyway, and the operator is better told nothing.
        assert_eq!(Secret::new("abc123").masked(), "••••");
        assert_eq!(Secret::new("12345678").masked(), "••••");
        assert_eq!(Secret::new("123456789").masked(), "••••6789");
    }

    #[test]
    fn byok_is_refused_for_the_services_whose_licence_is_ours() {
        assert!(Credentials::check_byok_allowed(Service::Deepgram).is_ok());
        assert!(Credentials::check_byok_allowed(Service::Anthropic).is_ok());

        for service in [Service::YouVersion, Service::TyndaleNlt] {
            let err = Credentials::check_byok_allowed(service)
                .expect_err("managed-only services must refuse BYOK");
            let message = err.to_string();
            // The refusal has to say why, or it reads as an arbitrary
            // limitation and someone will try to work around it.
            assert!(message.contains(service.label()), "{message}");
            assert!(message.contains("licence"), "{message}");
        }
    }

    #[test]
    fn only_active_states_let_a_client_call() {
        assert!(CredentialStatus::ManagedActive.is_usable());
        assert!(CredentialStatus::ByokActive {
            masked: "••••1234".into()
        }
        .is_usable());

        for status in [
            CredentialStatus::NotActivated,
            CredentialStatus::TokenRevoked,
            CredentialStatus::QuotaReached,
            CredentialStatus::GatewayUnreachable,
            CredentialStatus::DevKeyMissing {
                env_var: "DEEPGRAM_API_KEY".into(),
            },
        ] {
            assert!(!status.is_usable(), "{status:?} should not be usable");
        }
    }
}
