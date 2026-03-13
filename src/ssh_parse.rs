/// Parse a raw SSH command string into its components.
///
/// Handles all of these formats:
///   ssh user@host
///   ssh user@host -p 2222
///   ssh -p 2222 user@host
///   ssh -i ~/.ssh/key.pem user@host
///   ssh -p 2222 -i key.pem user@host
///   user@host                          (no leading "ssh")
///   host                               (no user — uses current $USER)
///   ssh -p22 root@host                 (port glued to flag)
#[derive(Debug, Default)]
pub struct ParsedSsh {
    pub username: String,
    pub host:     String,
    pub port:     u16,
    pub key_path: Option<String>,
}

impl ParsedSsh {
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }

        // Strip leading "ssh " (case-insensitive)
        let s = if raw.to_lowercase().starts_with("ssh ") {
            raw[4..].trim()
        } else if raw.eq_ignore_ascii_case("ssh") {
            return None;
        } else {
            raw
        };

        let tokens: Vec<&str> = s.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }

        let mut port: u16 = 22;
        let mut key_path: Option<String> = None;
        let mut destination: Option<&str> = None;

        let mut i = 0;
        while i < tokens.len() {
            let t = tokens[i];

            if t.starts_with('-') {
                // Flag token
                match t {
                    "-p" => {
                        i += 1;
                        if let Some(v) = tokens.get(i) {
                            port = v.parse().unwrap_or(22);
                        }
                    }
                    "-i" => {
                        i += 1;
                        if let Some(v) = tokens.get(i) {
                            key_path = Some(expand_tilde(v));
                        }
                    }
                    // -p22 (port glued)
                    t if t.starts_with("-p") && t.len() > 2 => {
                        port = t[2..].parse().unwrap_or(22);
                    }
                    // -i/path/to/key (glued)
                    t if t.starts_with("-i") && t.len() > 2 => {
                        key_path = Some(expand_tilde(&t[2..]));
                    }
                    // Flags we know are single-arg (ignore the value)
                    "-l" | "-o" | "-b" | "-c" | "-D" | "-E" | "-e" |
                    "-F" | "-I" | "-J" | "-L" | "-m" | "-O" | "-Q" |
                    "-R" | "-S" | "-W" | "-w" => {
                        i += 1; // skip next token (the value)
                    }
                    // Boolean flags — skip
                    _ => {}
                }
            } else if destination.is_none() {
                // First non-flag token is the destination (user@host or host)
                destination = Some(t);
            }
            // Extra non-flag tokens after destination are the remote command — ignore

            i += 1;
        }

        let dest = destination?;
        let (username, host) = if let Some(at) = dest.find('@') {
            (dest[..at].to_string(), dest[at + 1..].to_string())
        } else {
            // No user@ — default to current $USER
            let user = std::env::var("USER").unwrap_or_else(|_| "root".into());
            (user, dest.to_string())
        };

        if host.is_empty() {
            return None;
        }

        Some(Self { username, host, port, key_path })
    }
}

fn expand_tilde(path: &str) -> String {
    shellexpand::tilde(path).into_owned()
}

// ── Tests ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> ParsedSsh { ParsedSsh::parse(s).expect("should parse") }

    #[test]
    fn basic() {
        let r = p("ssh root@10.0.0.1");
        assert_eq!(r.host, "10.0.0.1");
        assert_eq!(r.username, "root");
        assert_eq!(r.port, 22);
    }

    #[test]
    fn with_port_after() {
        let r = p("ssh user@host -p 2222");
        assert_eq!(r.port, 2222);
    }

    #[test]
    fn with_port_before() {
        let r = p("ssh -p 2222 user@host");
        assert_eq!(r.port, 2222);
    }

    #[test]
    fn with_key() {
        let r = p("ssh -i ~/.ssh/id_rsa user@host");
        assert!(r.key_path.is_some());
        assert!(r.key_path.unwrap().contains("id_rsa"));
    }

    #[test]
    fn no_ssh_prefix() {
        let r = p("admin@192.168.1.100");
        assert_eq!(r.username, "admin");
        assert_eq!(r.host, "192.168.1.100");
    }

    #[test]
    fn host_only() {
        let r = p("myserver.lan");
        assert_eq!(r.host, "myserver.lan");
    }

    #[test]
    fn port_glued() {
        let r = p("ssh -p22 root@host");
        assert_eq!(r.port, 22);
    }

    #[test]
    fn full_complex() {
        let r = p("ssh -p 4422 -i ~/.ssh/prod.pem deploy@prod.example.com");
        assert_eq!(r.host, "prod.example.com");
        assert_eq!(r.username, "deploy");
        assert_eq!(r.port, 4422);
        assert!(r.key_path.unwrap().contains("prod.pem"));
    }
}
