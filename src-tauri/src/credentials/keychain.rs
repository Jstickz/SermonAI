//! BYOK storage: a church's own vendor keys, in the OS credential store.
//!
//! Windows Credential Manager or the macOS Keychain, through the `keyring`
//! crate. Phase 2 item 3 of `docs/SermonAI_API_Key_Strategy_Prompt.md` is
//! explicit about where these must *not* go: not SQLite, not logs, not crash
//! reports, not the settings export, and not an IPC payload to the frontend.
//!
//! ## Why the OS store rather than a file we encrypt
//!
//! Encrypting a file ourselves means shipping the decryption key inside the
//! same binary, which is obfuscation rather than protection. The OS store ties
//! the secret to the Windows user account, so another user on the same machine
//! cannot read it and a copied AppData folder is useless. It also means a key
//! survives an uninstall-reinstall, which matters on a church machine where the
//! person who pasted the key is rarely the person reinstalling.
//!
//! ## What leaves this module
//!
//! A [`Secret`], which masks itself in `Debug` and `Display`, or a `String`
//! that is already masked. The raw key goes to exactly one place: an
//! `Authorization` header, built at the point of use.

use keyring::Entry;

use super::{Secret, Service};
use crate::error::{Error, Result};

/// The credential-store namespace. Appears in Windows Credential Manager as
/// `SermonAI/deepgram` and similar, so an administrator auditing a machine can
/// see what we hold without a tool.
const NAMESPACE: &str = "SermonAI";

/// The OS credential store, scoped to one namespace.
///
/// Holds no secrets itself — every call opens an entry, reads or writes, and
/// drops it. Caching a key in memory would put it in a crash dump, and the
/// store is a local call measured in microseconds.
pub struct KeychainStore {
    namespace: String,
}

impl KeychainStore {
    pub fn new() -> Self {
        Self {
            namespace: NAMESPACE.to_string(),
        }
    }

    /// A store under its own namespace, so a test cannot read, overwrite or
    /// delete the developer's real keys on the machine running it.
    #[cfg(test)]
    pub fn with_namespace(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
        }
    }

    fn entry(&self, service: Service) -> Result<Entry> {
        Entry::new(&self.namespace, service.slug()).map_err(|err| {
            Error::Config(format!(
                "Could not open the credential store for {}: {err}. Sign in to Windows normally and try again.",
                service.label()
            ))
        })
    }

    /// The stored key, or `None` if this service has none.
    ///
    /// A missing entry is not an error — it is the ordinary state of a machine
    /// before onboarding, and treating it as a failure would make a fresh
    /// install look broken.
    pub fn get(&self, service: Service) -> Result<Option<Secret>> {
        match self.entry(service)?.get_password() {
            Ok(value) => {
                // An entry holding whitespace is one someone cleared by hand or
                // a write that half-succeeded. Passing it on buys a 401 from
                // the vendor instead of "no key set", which sends the operator
                // to their account page rather than to the paste box.
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(Secret::new(trimmed)))
                }
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(Error::Config(format!(
                "Could not read the stored {} key: {err}. Remove it in Settings and paste it again.",
                service.label()
            ))),
        }
    }

    /// Store a key, replacing any existing one.
    ///
    /// Rejects a blank key here rather than at the UI, so the rule holds for
    /// any future caller — an importer, a command-line flag — and not only for
    /// the settings panel that happens to call it today.
    pub fn set(&self, service: Service, key: &str) -> Result<()> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(Error::Config(format!(
                "No {} key was entered. Paste the key and save again.",
                service.label()
            )));
        }

        self.entry(service)?.set_password(trimmed).map_err(|err| {
            Error::Config(format!(
                "Could not save the {} key: {err}. Check that Windows Credential Manager is available.",
                service.label()
            ))
        })
    }

    /// Delete a key. Removing one that is not there succeeds: the operator
    /// asked for it to be gone, and it is.
    pub fn remove(&self, service: Service) -> Result<()> {
        match self.entry(service)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(Error::Config(format!(
                "Could not remove the {} key: {err}. Remove it from Windows Credential Manager directly.",
                service.label()
            ))),
        }
    }
}

impl Default for KeychainStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A namespace per test run, so these never touch the real `SermonAI`
    /// entries on a developer's machine and cannot collide with each other.
    fn scratch(name: &str) -> KeychainStore {
        KeychainStore::with_namespace(format!("SermonAI-test-{name}-{}", std::process::id()))
    }

    #[test]
    fn a_key_round_trips_and_can_be_removed() {
        let _store_lock = super::super::credential_store_lock();
        let store = scratch("roundtrip");

        assert!(
            store.get(Service::Deepgram).expect("read").is_none(),
            "a fresh namespace must start empty"
        );

        store
            .set(Service::Deepgram, "dg_test_abcdefghijkl")
            .expect("save");

        let found = store.get(Service::Deepgram).expect("read").expect("a key");
        assert_eq!(found.expose(), "dg_test_abcdefghijkl");
        assert_eq!(found.masked(), "••••ijkl");

        store.remove(Service::Deepgram).expect("remove");
        assert!(store.get(Service::Deepgram).expect("read").is_none());

        // Removing what is already gone is what the operator asked for.
        store.remove(Service::Deepgram).expect("remove again");
    }

    #[test]
    fn services_do_not_share_an_entry() {
        // They are stored under one namespace and differ only by slug, so a
        // slug collision would silently let one service's key answer for
        // another's — and the symptom would be a 401 from the wrong vendor.
        let _store_lock = super::super::credential_store_lock();
        let store = scratch("isolation");
        store
            .set(Service::Deepgram, "dg_one_aaaaaaaa")
            .expect("save");
        store
            .set(Service::Anthropic, "sk_two_bbbbbbbb")
            .expect("save");

        assert_eq!(
            store.get(Service::Deepgram).unwrap().unwrap().expose(),
            "dg_one_aaaaaaaa"
        );
        assert_eq!(
            store.get(Service::Anthropic).unwrap().unwrap().expose(),
            "sk_two_bbbbbbbb"
        );

        store.remove(Service::Deepgram).expect("remove");
        assert!(store.get(Service::Deepgram).unwrap().is_none());
        assert!(
            store.get(Service::Anthropic).unwrap().is_some(),
            "removing one service must not remove another"
        );

        store.remove(Service::Anthropic).expect("cleanup");
    }

    #[test]
    fn whitespace_is_trimmed_on_the_way_in_and_treated_as_absent() {
        // Pasting from a browser or a password manager routinely carries a
        // trailing newline, and a key with one produces a 401 that reads as a
        // wrong key rather than a stray character.
        let _store_lock = super::super::credential_store_lock();
        let store = scratch("whitespace");

        store
            .set(Service::Anthropic, "  sk_padded_cccccccc\n")
            .expect("save");
        assert_eq!(
            store.get(Service::Anthropic).unwrap().unwrap().expose(),
            "sk_padded_cccccccc"
        );

        let err = store
            .set(Service::Anthropic, "   ")
            .expect_err("a blank key is not a key");
        assert!(err.to_string().contains("Anthropic"), "{err}");

        store.remove(Service::Anthropic).expect("cleanup");
    }

    #[test]
    fn every_service_has_a_distinct_slug() {
        let mut seen = std::collections::HashSet::new();
        for service in Service::all() {
            assert!(
                seen.insert(service.slug()),
                "{} reuses a slug, which would share a credential entry",
                service.label()
            );
        }
    }
}
