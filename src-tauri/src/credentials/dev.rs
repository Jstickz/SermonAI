//! The development credential provider: keys from the environment.
//!
//! This is the *only* place in the app that reads a vendor key from an
//! environment variable, and it exists so that Phase 3's gateway provider can
//! replace it without any service client noticing.
//!
//! ## It cannot run in a shipped app
//!
//! [`DevProvider::new`] returns `None` outside `debug_assertions`, and the
//! lookup body is compiled out too. A release build therefore has no path from
//! an environment variable to a vendor call — not a path that finds nothing,
//! but no path at all. That is deliberate: setting `DEEPGRAM_API_KEY` in a
//! church's system environment must not quietly turn into billed usage on
//! somebody's account.
//!
//! Until the gateway exists, a release build consequently has no credentials at
//! all and reports [`CredentialStatus::NotActivated`]. That is correct rather
//! than convenient, and it is the gate on M4's first Sunday.

use super::{Access, CredentialStatus, Provider, Secret, Service};
use crate::error::{Error, Result};

pub struct DevProvider;

impl DevProvider {
    /// A provider, or `None` in a release build.
    ///
    /// The caller decides what to do with `None`; `AppState` falls back to a
    /// provider that reports nothing is activated.
    pub fn new() -> Option<Self> {
        if cfg!(debug_assertions) {
            Some(Self)
        } else {
            None
        }
    }

    fn key(service: Service) -> Option<Secret> {
        #[cfg(debug_assertions)]
        {
            let raw = std::env::var(service.dev_env_var()).ok()?;
            // An empty or whitespace variable is an unset one that happens to
            // exist — a blank line in .env, or `export KEY=` — and treating it
            // as a key produces a 401 from the vendor instead of a message
            // saying which variable to fill in.
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return None;
            }
            Some(Secret::new(trimmed))
        }

        #[cfg(not(debug_assertions))]
        {
            let _ = service;
            None
        }
    }
}

impl Provider for DevProvider {
    fn access(&self, service: Service) -> Result<Access> {
        match Self::key(service) {
            Some(secret) => Ok(Access::DirectKey(secret)),
            None => Err(Error::Config(format!(
                "{} is not configured. Add {} to your .env and restart.",
                service.label(),
                service.dev_env_var()
            ))),
        }
    }

    fn status(&self, service: Service) -> CredentialStatus {
        match Self::key(service) {
            // Reported as BYOK rather than managed: a development key is
            // literally a key held on this machine, which is what BYOK means.
            // Calling it managed would make the status screen lie about where
            // the credential lives.
            Some(secret) => CredentialStatus::ByokActive {
                masked: secret.masked(),
            },
            None => CredentialStatus::DevKeyMissing {
                env_var: service.dev_env_var().to_string(),
            },
        }
    }
}

/// Used when no provider is available: every release build until the gateway
/// lands, and any dev build where `DevProvider::new` declined.
pub struct UnconfiguredProvider;

impl Provider for UnconfiguredProvider {
    fn access(&self, service: Service) -> Result<Access> {
        Err(Error::Config(format!(
            "{} is not available yet. SermonAI has not been activated on this machine.",
            service.label()
        )))
    }

    fn status(&self, _service: Service) -> CredentialStatus {
        CredentialStatus::NotActivated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Env vars are process-global, so these run under one lock rather than
    /// racing each other into confusing failures.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct EnvGuard {
        var: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(var: &'static str, value: Option<&str>) -> Self {
            let previous = std::env::var(var).ok();
            match value {
                Some(v) => std::env::set_var(var, v),
                None => std::env::remove_var(var),
            }
            Self { var, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(v) => std::env::set_var(self.var, v),
                None => std::env::remove_var(self.var),
            }
        }
    }

    #[test]
    fn a_configured_key_is_returned_and_reported_masked() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::set("DEEPGRAM_API_KEY", Some("dg_test_abcdefghijkl"));

        let provider = DevProvider::new().expect("tests are a debug build");

        match provider.access(Service::Deepgram).expect("should resolve") {
            Access::DirectKey(secret) => assert_eq!(secret.expose(), "dg_test_abcdefghijkl"),
            Access::Gateway { .. } => panic!("the dev provider issues no gateway access"),
        }

        assert_eq!(
            provider.status(Service::Deepgram),
            CredentialStatus::ByokActive {
                masked: "••••ijkl".to_string()
            }
        );
    }

    #[test]
    fn a_blank_variable_counts_as_missing() {
        // `export DEEPGRAM_API_KEY=` and a stray blank line in .env both reach
        // here. Passing the empty string on would produce a 401 from the
        // vendor rather than a message naming the variable to fill in.
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::set("ANTHROPIC_API_KEY", Some("   "));

        let provider = DevProvider::new().expect("tests are a debug build");

        assert_eq!(
            provider.status(Service::Anthropic),
            CredentialStatus::DevKeyMissing {
                env_var: "ANTHROPIC_API_KEY".to_string()
            }
        );
    }

    #[test]
    fn a_missing_key_names_the_variable_to_set() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::set("TYNDALE_API_KEY", None);

        let provider = DevProvider::new().expect("tests are a debug build");
        let message = provider
            .access(Service::TyndaleNlt)
            .expect_err("no key means no access")
            .to_string();

        assert!(message.contains("TYNDALE_API_KEY"), "{message}");
        assert!(message.contains("Tyndale NLT"), "{message}");
    }

    #[test]
    fn an_unconfigured_machine_says_so_rather_than_failing_obscurely() {
        let provider = UnconfiguredProvider;

        assert_eq!(
            provider.status(Service::Deepgram),
            CredentialStatus::NotActivated
        );
        let message = provider
            .access(Service::Deepgram)
            .expect_err("nothing is configured")
            .to_string();
        assert!(message.contains("activated"), "{message}");
    }
}
