//! Thin wrapper around the OS keyring for storing SSH passwords.
//! Service name: "shellkeeper"  — entry key: connection UUID

const SERVICE: &str = "shellkeeper";

/// Store a password in the OS keyring for the given connection id.
pub fn set_password(conn_id: &str, password: &str) -> bool {
    match keyring::Entry::new(SERVICE, conn_id) {
        Ok(entry) => entry.set_password(password).is_ok(),
        Err(_) => false,
    }
}

/// Retrieve a previously stored password. Returns None if not found or keyring unavailable.
pub fn get_password(conn_id: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, conn_id)
        .ok()?
        .get_password()
        .ok()
}

/// Delete a stored password (e.g. when connection is deleted or user unchecks save).
pub fn delete_password(conn_id: &str) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, conn_id) {
        let _ = entry.delete_credential();
    }
}
