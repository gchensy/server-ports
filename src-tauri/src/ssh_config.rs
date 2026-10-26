//! The servers of an OpenSSH client configuration (`~/.ssh/config`), turned into profiles. The
//! file is only ever read.
//!
//! A host is resolved the way `ssh -G <name>` does it: the blocks whose `Host` patterns match are
//! applied in file order and the first value of an option wins (so `Host *` gives defaults when
//! it comes last and overrides when it comes first). `LocalForward` is the one option that adds
//! up. `Include` is followed, `Match` blocks are skipped, and a line that cannot be understood is
//! ignored.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::model::{Auth, Forward, Profile};

/// What OpenSSH allows, and far deeper than any real configuration goes.
const MAX_INCLUDE_DEPTH: usize = 16;

/// Who is reading the configuration: what `~`, `Include` and the `%d` and `%u` tokens refer to.
#[derive(Debug, Clone)]
pub struct LocalUser {
    pub name: String,
    pub home: PathBuf,
}

impl LocalUser {
    /// Whoever runs this process.
    pub fn current() -> Self {
        let name = ["USER", "LOGNAME", "USERNAME"]
            .into_iter()
            .find_map(|var| std::env::var(var).ok().filter(|name| !name.is_empty()))
            .unwrap_or_default();
        Self {
            name,
            home: dirs::home_dir().unwrap_or_default(),
        }
    }

    fn ssh_dir(&self) -> PathBuf {
        self.home.join(".ssh")
    }

    /// An `Include` argument as a path; a relative one starts in `~/.ssh`.
    fn locate(&self, arg: &str) -> PathBuf {
        match arg.strip_prefix("~/").or_else(|| arg.strip_prefix("~\\")) {
            Some(rest) => self.home.join(rest),
            None => self.ssh_dir().join(arg),
        }
    }
}

/// How a host is reached when it is not reached directly: something the engine cannot do yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Proxy {
    ProxyJump,
    ProxyCommand,
}

/// One server of the configuration, as the profile it would become.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHost {
    /// Not saved yet (the id is empty). It can still be unusable as it is, for instance without
    /// a single `LocalForward`.
    pub profile: Profile,
    pub proxy: Option<Proxy>,
}

/// Which hosts a run of settings is for.
#[derive(Debug, Clone)]
enum Scope {
    /// Before the first `Host` line.
    Everywhere,
    /// The patterns of a `Host` line, `!` and all.
    Hosts(Vec<String>),
    /// A `Match` block. Its conditions are not evaluated, so none of it applies.
    Nowhere,
}

impl Scope {
    fn covers(&self, name: &str) -> bool {
        let Scope::Hosts(patterns) = self else {
            return matches!(self, Scope::Everywhere);
        };
        let mut covered = false;
        for pattern in patterns {
            match pattern.strip_prefix('!') {
                Some(excluded) if wildcard_match(excluded, name) => return false,
                Some(_) => {}
                None => covered |= wildcard_match(pattern, name),
            }
        }
        covered
    }
}

/// A run of settings (lower-cased keyword, arguments) that share a scope.
#[derive(Debug)]
struct Block {
    scope: Scope,
    settings: Vec<(String, Vec<String>)>,
}

/// The configuration file and the files it includes, read once.
#[derive(Debug, Default)]
pub struct SshConfig {
    blocks: Vec<Block>,
    /// What `Host` lines call single servers (no wildcard, no `!`), in the order they appear.
    aliases: Vec<String>,
}

impl SshConfig {
    /// Reads `~/.ssh/config` and what it includes. A missing file is an empty configuration;
    /// one that exists but cannot be read is an error.
    pub fn load(user: &LocalUser) -> std::io::Result<Self> {
        let path = user.ssh_dir().join("config");
        let text = match read_text(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => {
                return Err(std::io::Error::new(
                    e.kind(),
                    format!("{}: {e}", path.display()),
                ));
            }
        };
        let mut config = Self::default();
        config.parse(&text, user, Scope::Everywhere, &mut vec![path]);
        Ok(config)
    }

    /// Every single server the file names, in the order it names them.
    pub fn hosts(&self, user: &LocalUser) -> Vec<SshHost> {
        self.aliases
            .iter()
            .map(|alias| self.resolve(alias, user))
            .collect()
    }

    /// `chain` holds the files being read, from the outermost in.
    fn parse(&mut self, text: &str, user: &LocalUser, mut scope: Scope, chain: &mut Vec<PathBuf>) {
        // What an `Include` inside a `Match` block brings in can never apply, `Host` lines or not.
        let dead = matches!(scope, Scope::Nowhere);
        self.begin(&scope);
        for line in text.lines() {
            let Some((keyword, args)) = split_line(line) else {
                continue;
            };
            match keyword.as_str() {
                "host" => {
                    if !dead {
                        let single = |p: &&String| {
                            !p.is_empty() && !p.starts_with('!') && !p.contains(['*', '?'])
                        };
                        for alias in args.iter().filter(single) {
                            if !self.aliases.contains(alias) {
                                self.aliases.push(alias.clone());
                            }
                        }
                    }
                    scope = if dead {
                        Scope::Nowhere
                    } else {
                        Scope::Hosts(args)
                    };
                    self.begin(&scope);
                }
                "match" => {
                    scope = Scope::Nowhere;
                    self.begin(&scope);
                }
                "include" => {
                    for path in args.iter().flat_map(|arg| glob(&user.locate(arg))) {
                        self.include(&path, user, &scope, chain);
                    }
                    // What follows is in the scope it was before, whatever the files did.
                    self.begin(&scope);
                }
                _ => {
                    if let Some(block) = self.blocks.last_mut() {
                        block.settings.push((keyword, args));
                    }
                }
            }
        }
    }

    fn begin(&mut self, scope: &Scope) {
        self.blocks.push(Block {
            scope: scope.clone(),
            settings: Vec::new(),
        });
    }

    /// The included file starts in the scope of the `Include` line, so that an `Include` below a
    /// `Host` line gives the host the settings of the file.
    fn include(&mut self, path: &Path, user: &LocalUser, scope: &Scope, chain: &mut Vec<PathBuf>) {
        if chain.len() >= MAX_INCLUDE_DEPTH || chain.iter().any(|open| open == path) {
            return;
        }
        let Ok(text) = read_text(path) else {
            return;
        };
        chain.push(path.to_path_buf());
        self.parse(&text, user, scope.clone(), chain);
        chain.pop();
    }

    fn resolve(&self, alias: &str, user: &LocalUser) -> SshHost {
        let applying: Vec<_> = self
            .blocks
            .iter()
            .filter(|block| block.scope.covers(alias))
            .flat_map(|block| &block.settings)
            .collect();
        let all = |keyword: &str| -> Vec<&[String]> {
            applying
                .iter()
                .filter(|setting| setting.0 == keyword)
                .map(|setting| setting.1.as_slice())
                .collect()
        };
        let first = |keyword: &str| all(keyword).into_iter().find_map(|args| args.first());

        let host = first("hostname")
            .map_or_else(|| alias.to_string(), |name| expand(name, &[('h', alias)]));
        let username = first("user").map_or_else(|| user.name.clone(), String::clone);
        let port = all("port")
            .into_iter()
            .find_map(|args| args.first()?.parse::<u16>().ok().filter(|port| *port != 0))
            .unwrap_or(22);
        let auth = first("identityfile").map_or(Auth::Agent, |path| {
            let (home, port) = (user.home.to_string_lossy(), port.to_string());
            let tokens = [
                ('d', home.as_ref()),
                ('h', host.as_str()),
                ('n', alias),
                ('p', port.as_str()),
                ('r', username.as_str()),
                ('u', user.name.as_str()),
            ];
            Auth::Key {
                path: expand(path, &tokens),
            }
        });
        let mut forwards: Vec<Forward> = Vec::new();
        for forward in all("localforward").into_iter().filter_map(local_forward) {
            if !forwards.contains(&forward) {
                forwards.push(forward);
            }
        }
        let proxied =
            |keyword| first(keyword).is_some_and(|value| !value.eq_ignore_ascii_case("none"));
        let proxy = if proxied("proxyjump") {
            Some(Proxy::ProxyJump)
        } else if proxied("proxycommand") {
            Some(Proxy::ProxyCommand)
        } else {
            None
        };

        SshHost {
            profile: Profile {
                id: String::new(),
                name: alias.to_string(),
                host,
                port,
                username,
                auth,
                forwards,
                auto_reconnect: true,
                connect_on_launch: false,
            },
            proxy,
        }
    }
}

fn read_text(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    // Editors on Windows like to put a byte order mark in front.
    Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string())
}

/// The keyword (lower-cased) and the arguments of a line. `None` for a line with nothing in it.
/// The two may be separated by blanks, by `=`, or by both.
fn split_line(line: &str) -> Option<(String, Vec<String>)> {
    let line = line.trim_start();
    let (keyword, rest) = line.split_at(
        line.find(|c: char| c.is_whitespace() || c == '=')
            .unwrap_or(line.len()),
    );
    if keyword.is_empty() || keyword.starts_with('#') {
        return None;
    }
    let rest = rest.trim_start();
    Some((
        keyword.to_ascii_lowercase(),
        arguments(rest.strip_prefix('=').unwrap_or(rest)),
    ))
}

/// Splits on blanks the way OpenSSH does: single and double quotes group (and vanish), a
/// backslash protects a quote, another backslash or (outside quotes) a blank, and a `#` where an
/// argument would begin starts a comment.
fn arguments(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut chars = text.chars().peekable();
    loop {
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        if chars.peek().is_none_or(|c| *c == '#') {
            return args;
        }
        let mut arg = String::new();
        let mut quote = None;
        while let Some(c) = chars.next() {
            match (c, quote) {
                ('\\', _) => match chars
                    .next_if(|n| matches!(n, '\'' | '"' | '\\') || (quote.is_none() && *n == ' '))
                {
                    Some(escaped) => arg.push(escaped),
                    None => arg.push('\\'),
                },
                (c, None) if c.is_whitespace() => break,
                ('\'' | '"', None) => quote = Some(c),
                (c, Some(open)) if c == open => quote = None,
                _ => arg.push(c),
            }
        }
        args.push(arg);
    }
}

/// `LocalForward [bind_address:]port host:hostport`, which arrives as two arguments. `*` or
/// nothing in front of the port means every interface, as in OpenSSH. A form that cannot be a
/// forward of this app (a socket path, a port name) is dropped.
fn local_forward(args: &[String]) -> Option<Forward> {
    let [listen, target] = args else {
        return None;
    };
    let (bind, local_port) = host_and_port(listen)?;
    let (remote_host, remote_port) = host_and_port(target)?;
    let bind_host = match bind {
        None => "127.0.0.1",
        Some(host) if host.eq_ignore_ascii_case("localhost") => "127.0.0.1",
        Some("" | "*") => "0.0.0.0",
        Some(host) => host,
    };
    Some(Forward {
        id: String::new(),
        label: String::new(),
        bind_host: bind_host.to_string(),
        local_port,
        remote_host: remote_host.filter(|host| !host.is_empty())?.to_string(),
        remote_port,
    })
}

/// `port`, `host:port` or `[ipv6]:port`.
fn host_and_port(text: &str) -> Option<(Option<&str>, u16)> {
    let (host, port) = match text.rsplit_once(':') {
        Some((host, port)) => (Some(host), port),
        None => (None, text),
    };
    let host = host.map(|host| {
        host.strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
            .unwrap_or(host)
    });
    Some((host, port.parse().ok().filter(|port| *port != 0)?))
}

/// Replaces `%x` for each `(x, value)` and `%%` for `%`. Other tokens are left as they are.
fn expand(text: &str, tokens: &[(char, &str)]) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') | None => out.push('%'),
            Some(token) => match tokens.iter().find(|(name, _)| *name == token) {
                Some((_, value)) => out.push_str(value),
                None => out.extend(['%', token]),
            },
        }
    }
    out
}

/// Whether `text` fits `pattern`, in which `*` stands for any run of characters and `?` for one.
fn wildcard_match(pattern: &str, text: &str) -> bool {
    match pattern.chars().next() {
        None => text.is_empty(),
        Some('*') => (0..=text.len())
            .filter(|&end| text.is_char_boundary(end))
            .any(|end| wildcard_match(&pattern[1..], &text[end..])),
        Some(c) => text.chars().next().is_some_and(|t| {
            (c == '?' || c == t) && wildcard_match(&pattern[c.len_utf8()..], &text[t.len_utf8()..])
        }),
    }
}

/// The existing paths that match `pattern`, in alphabetical order. `*` and `?` are the wildcards,
/// and as in a shell they do not match a dot at the start of a name.
fn glob(pattern: &Path) -> Vec<PathBuf> {
    let mut found = vec![PathBuf::new()];
    for part in pattern.components() {
        let part = part.as_os_str();
        let wild = part.to_string_lossy().contains(['*', '?']);
        if !wild {
            found.iter_mut().for_each(|path| path.push(part));
            continue;
        }
        let pattern = part.to_string_lossy();
        let mut next = Vec::new();
        for dir in &found {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            let mut names: Vec<_> = entries
                .flatten()
                .map(|entry| entry.file_name())
                .filter(|name| {
                    let name = name.to_string_lossy();
                    (pattern.starts_with('.') || !name.starts_with('.'))
                        && wildcard_match(&pattern, &name)
                })
                .collect();
            names.sort();
            next.extend(names.into_iter().map(|name| dir.join(name)));
        }
        found = next;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A home directory with a `.ssh/config` and whatever other files a test adds. All the names,
    /// users and addresses in the fixtures are made up.
    struct Home(tempfile::TempDir);

    impl Home {
        fn new(config: &str) -> Self {
            let home = Self(tempfile::tempdir().unwrap());
            home.file(".ssh/config", config);
            home
        }

        fn file(&self, path: &str, text: &str) {
            let path = self.0.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }

        fn user(&self) -> LocalUser {
            LocalUser {
                name: "alice".into(),
                home: self.0.path().to_path_buf(),
            }
        }

        fn hosts(&self) -> Vec<SshHost> {
            let user = self.user();
            SshConfig::load(&user).unwrap().hosts(&user)
        }
    }

    fn hosts(config: &str) -> Vec<SshHost> {
        Home::new(config).hosts()
    }

    fn names(hosts: &[SshHost]) -> Vec<&str> {
        hosts.iter().map(|h| h.profile.name.as_str()).collect()
    }

    fn host<'a>(hosts: &'a [SshHost], name: &str) -> &'a SshHost {
        hosts
            .iter()
            .find(|h| h.profile.name == name)
            .unwrap_or_else(|| panic!("no host called {name} in {:?}", names(hosts)))
    }

    /// Where each forward of the profile listens and what it leads to.
    fn routes(profile: &Profile) -> Vec<(&str, u16, &str, u16)> {
        profile
            .forwards
            .iter()
            .map(|f| {
                (
                    f.bind_host.as_str(),
                    f.local_port,
                    f.remote_host.as_str(),
                    f.remote_port,
                )
            })
            .collect()
    }

    #[test]
    fn a_host_becomes_a_profile() {
        let hosts = hosts(
            "Host web
                 HostName web.example.com
                 User deploy
                 Port 2222
                 IdentityFile ~/.ssh/web_ed25519
                 LocalForward 8080 localhost:80
                 LocalForward 127.0.0.2:8443 10.0.0.5:443
            ",
        );
        assert_eq!(names(&hosts), ["web"]);
        assert_eq!(hosts[0].proxy, None);
        let p = &hosts[0].profile;
        assert!(p.id.is_empty(), "it is not saved yet");
        assert_eq!(
            (p.host.as_str(), p.port, p.username.as_str()),
            ("web.example.com", 2222, "deploy")
        );
        assert_eq!(
            p.auth,
            Auth::Key {
                path: "~/.ssh/web_ed25519".into()
            }
        );
        assert_eq!(
            routes(p),
            [
                ("127.0.0.1", 8080, "localhost", 80),
                ("127.0.0.2", 8443, "10.0.0.5", 443)
            ]
        );
        assert!(p.auto_reconnect && !p.connect_on_launch);
    }

    #[test]
    fn what_the_file_leaves_out_is_filled_in() {
        let hosts = hosts("Host plain.example.com\n");
        let p = &hosts[0].profile;
        assert_eq!(p.name, "plain.example.com");
        assert_eq!(
            (p.host.as_str(), p.port, p.username.as_str()),
            ("plain.example.com", 22, "alice")
        );
        assert_eq!(p.auth, Auth::Agent);
        assert!(p.forwards.is_empty());
    }

    #[test]
    fn keywords_ignore_case_and_values_may_be_quoted_or_follow_an_equals_sign() {
        let hosts = hosts(
            "host=quoted
               HOSTNAME = \"quoted.example.com\"
               PORT=2200
               User 'bob'
               identityfile \"~/my keys/id rsa\"
               LocalForward=8080 localhost:80
            ",
        );
        let p = &hosts[0].profile;
        assert_eq!(p.name, "quoted");
        assert_eq!(
            (p.host.as_str(), p.port, p.username.as_str()),
            ("quoted.example.com", 2200, "bob")
        );
        assert_eq!(
            p.auth,
            Auth::Key {
                path: "~/my keys/id rsa".into()
            }
        );
        assert_eq!(routes(p), [("127.0.0.1", 8080, "localhost", 80)]);
    }

    #[test]
    fn comments_blank_and_malformed_lines_are_ignored() {
        let hosts = hosts(
            "# Host commented
               # an indented comment

             Host web   # a comment after the patterns
               HostName web.example.com # and after a value
               Port nonsense
               Port 0
               Port 70000
               Port 2022
               LocalForward 8080
               LocalForward 8080 localhost
               LocalForward x localhost:80
               LocalForward 9090 localhost:90
               Bogus
               =
             Host
            ",
        );
        assert_eq!(names(&hosts), ["web"]);
        let p = &hosts[0].profile;
        assert_eq!((p.host.as_str(), p.port), ("web.example.com", 2022));
        assert_eq!(routes(p), [("127.0.0.1", 9090, "localhost", 90)]);
    }

    #[test]
    fn host_lines_name_each_single_server_once() {
        let hosts = hosts("Host a b *.example.com !c d?\nHost a\nHost !e f\n");
        assert_eq!(names(&hosts), ["a", "b", "f"]);
    }

    #[test]
    fn wildcard_blocks_give_defaults_and_the_first_value_wins() {
        let hosts = hosts(
            "Host app
               User deploy
               LocalForward 8080 localhost:80

             Host *.internal
               HostName 10.0.0.9

             Host *
               User fallback
               Port 2200
               IdentityFile ~/.ssh/default
               LocalForward 9000 localhost:9000

             Host web.internal
            ",
        );
        assert_eq!(names(&hosts), ["app", "web.internal"]);
        let app = &host(&hosts, "app").profile;
        assert_eq!((app.username.as_str(), app.port), ("deploy", 2200));
        assert_eq!(
            app.auth,
            Auth::Key {
                path: "~/.ssh/default".into()
            }
        );
        assert_eq!(
            routes(app),
            [
                ("127.0.0.1", 8080, "localhost", 80),
                ("127.0.0.1", 9000, "localhost", 9000)
            ],
            "forwards add up"
        );
        let web = &host(&hosts, "web.internal").profile;
        assert_eq!(
            (web.host.as_str(), web.username.as_str()),
            ("10.0.0.9", "fallback")
        );
    }

    #[test]
    fn a_wildcard_block_in_front_is_a_rule_not_a_default() {
        let hosts = hosts("Host *\n  User first\nHost later\n  User second\n");
        assert_eq!(host(&hosts, "later").profile.username, "first");
    }

    #[test]
    fn a_negated_pattern_leaves_a_host_out_of_the_block() {
        let hosts =
            hosts("Host * !secret\n  User common\nHost secret\n  User special\nHost open\n");
        assert_eq!(host(&hosts, "secret").profile.username, "special");
        assert_eq!(host(&hosts, "open").profile.username, "common");
    }

    #[test]
    fn match_blocks_are_skipped() {
        let hosts = hosts(
            "Host web
               User web-user
             Match host web
               User from-match
               Port 2222
             Host db
               HostName db.example.com
             Match all
               User from-match-all
            ",
        );
        let web = &host(&hosts, "web").profile;
        assert_eq!((web.username.as_str(), web.port), ("web-user", 22));
        assert_eq!(host(&hosts, "db").profile.username, "alice");
    }

    #[test]
    fn include_is_followed_in_place() {
        let home = Home::new("");
        let elsewhere = home.0.path().join("elsewhere/file");
        home.file(
            ".ssh/config",
            &format!(
                "Include conf.d/*.conf lima/*/ssh.config\nInclude ~/extra/more {}\nInclude missing/* nothing-here\n\nHost main\n",
                elsewhere.to_string_lossy().replace('\\', "/")
            ),
        );
        home.file(".ssh/conf.d/20-b.conf", "Host b\n  User from-b\n");
        home.file(".ssh/conf.d/10-a.conf", "Host a\n  User from-a\n");
        home.file(".ssh/conf.d/.hidden.conf", "Host hidden\n");
        home.file(".ssh/conf.d/notes.txt", "Host notes\n");
        home.file(".ssh/lima/one/ssh.config", "Host one\n");
        home.file(".ssh/lima/two/ssh.config", "Host two\n");
        home.file("extra/more", "Host more\n");
        home.file("elsewhere/file", "Host far\n");

        let hosts = home.hosts();
        assert_eq!(
            names(&hosts),
            ["a", "b", "one", "two", "more", "far", "main"]
        );
        assert_eq!(host(&hosts, "a").profile.username, "from-a");
    }

    #[test]
    fn an_include_below_a_host_line_gives_the_host_the_settings_of_the_file() {
        let home = Home::new("Host x\n  Include shared\nHost y\n  Include shared\nHost z\n");
        home.file(".ssh/shared", "User shared-user\nPort 2022\n");
        let hosts = home.hosts();
        for name in ["x", "y"] {
            let p = &host(&hosts, name).profile;
            assert_eq!(
                (p.username.as_str(), p.port),
                ("shared-user", 2022),
                "{name}"
            );
        }
        let z = &host(&hosts, "z").profile;
        assert_eq!((z.username.as_str(), z.port), ("alice", 22));
    }

    #[test]
    fn nothing_in_a_file_included_from_a_match_block_applies() {
        let home = Home::new("Match all\n  Include shared\nHost x\n");
        home.file(".ssh/shared", "User never\nHost hidden\n  User never\n");
        let hosts = home.hosts();
        assert_eq!(names(&hosts), ["x"]);
        assert_eq!(hosts[0].profile.username, "alice");
    }

    #[test]
    fn an_include_can_neither_loop_nor_go_on_for_ever() {
        let home = Home::new("Include loop\nHost main\n");
        home.file(".ssh/loop", "Include loop\nHost looped\n");
        assert_eq!(names(&home.hosts()), ["looped", "main"]);

        let home = Home::new("Include level0\nHost main\n");
        for level in 0..MAX_INCLUDE_DEPTH + 4 {
            home.file(
                &format!(".ssh/level{level}"),
                &format!("Include level{}\nHost deep{level}\n", level + 1),
            );
        }
        // The configuration is the first of the files being read, then come the levels.
        assert_eq!(home.hosts().len(), MAX_INCLUDE_DEPTH);
    }

    #[test]
    fn tokens_are_expanded_in_key_files_and_host_names() {
        let home = Home::new(
            "Host tok
               HostName tok.example.com
               User bob
               IdentityFile %d/keys/%h_%r_%u_%n_%p_%%_%x
             Host api.corp
             Host *.corp
               HostName %h.example.org
            ",
        );
        let hosts = home.hosts();
        let Auth::Key { path } = &host(&hosts, "tok").profile.auth else {
            panic!("expected a key file");
        };
        let expected = format!(
            "{}/keys/tok.example.com_bob_alice_tok_22_%_%x",
            home.0.path().display()
        );
        assert_eq!(path, &expected, "unknown tokens stay as they are");
        assert_eq!(
            host(&hosts, "api.corp").profile.host,
            "api.corp.example.org"
        );
    }

    #[test]
    fn local_forwards_come_in_every_form_of_the_manual() {
        let hosts = hosts(
            "Host fw
               LocalForward 8080 localhost:80
               LocalForward *:8081 db.internal:5432
               LocalForward :8082 db.internal:5433
               LocalForward LocalHost:8083 10.0.0.1:81
               LocalForward [::1]:8084 [fe80::1]:82
               LocalForward 0.0.0.0:8085 example.com:83
               LocalForward 8080 localhost:80
               LocalForward 8086 example.com:0
               LocalForward 99999 example.com:84
               LocalForward 8087 /var/run/service.sock
               RemoteForward 7000 localhost:7000
               DynamicForward 1080
            ",
        );
        assert_eq!(
            routes(&hosts[0].profile),
            [
                ("127.0.0.1", 8080, "localhost", 80),
                ("0.0.0.0", 8081, "db.internal", 5432),
                ("0.0.0.0", 8082, "db.internal", 5433),
                ("127.0.0.1", 8083, "10.0.0.1", 81),
                ("::1", 8084, "fe80::1", 82),
                ("0.0.0.0", 8085, "example.com", 83),
            ],
            "an exact repeat, bad ports and sockets are dropped, the other kinds of forward are not ours"
        );
    }

    #[test]
    fn imported_hosts_pass_the_checks_of_the_app() {
        let hosts = hosts("Host ok\n  LocalForward 8080 localhost:80\nHost bare\n");
        let mut ok = hosts[0].profile.clone();
        ok.normalize().unwrap();
        assert_eq!(ok.forwards[0].bind_host, "127.0.0.1");

        // A server needs a port mapping; the UI asks for one.
        assert_eq!(
            hosts[1].profile.clone().normalize().unwrap_err().code,
            "forwards"
        );
    }

    #[test]
    fn hosts_reached_through_a_proxy_are_marked() {
        let hosts = hosts(
            "Host jump
               ProxyJump bastion.example.com
             Host cmd
               ProxyCommand ssh -W %h:%p bastion.example.com
             Host direct
               ProxyJump none
             Host both
               ProxyCommand none
               ProxyJump bastion.example.com
             Host plain
            ",
        );
        let proxies: Vec<_> = hosts.iter().map(|h| h.proxy).collect();
        assert_eq!(
            proxies,
            [
                Some(Proxy::ProxyJump),
                Some(Proxy::ProxyCommand),
                None,
                Some(Proxy::ProxyJump),
                None
            ]
        );
        assert_eq!(serde_json::to_value(Proxy::ProxyJump).unwrap(), "ProxyJump");
    }

    #[test]
    fn byte_order_marks_and_windows_line_endings_are_harmless() {
        let hosts = hosts("\u{feff}Host a\r\n  HostName a.example.com\r\n  Port 2200\r\n");
        let p = &hosts[0].profile;
        assert_eq!(
            (p.name.as_str(), p.host.as_str(), p.port),
            ("a", "a.example.com", 2200)
        );
    }

    #[test]
    fn a_missing_config_is_empty_but_an_unreadable_one_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let user = LocalUser {
            name: "alice".into(),
            home: dir.path().to_path_buf(),
        };
        assert!(SshConfig::load(&user).unwrap().hosts(&user).is_empty());

        // A folder where the file should be.
        std::fs::create_dir_all(dir.path().join(".ssh/config")).unwrap();
        let error = SshConfig::load(&user).unwrap_err();
        assert!(error.to_string().contains("config"), "{error}");
    }

    #[test]
    fn wildcards_match_like_a_shell() {
        let cases = [
            ("*", "anything", true),
            ("*", "", true),
            ("web*", "web", true),
            ("web*", "webserver", true),
            ("web*", "my-web", false),
            ("*.example.com", "a.example.com", true),
            ("*.example.com", "example.com", false),
            ("web?", "web1", true),
            ("web?", "web", false),
            ("web?", "web12", false),
            ("a*b*c", "aXbYc", true),
            ("a*b*c", "aXbY", false),
            ("exact", "exact", true),
            ("exact", "Exact", false),
            ("?", "ü", true),
            ("ü*", "übung", true),
        ];
        for (pattern, text, fits) in cases {
            assert_eq!(
                wildcard_match(pattern, text),
                fits,
                "{pattern} against {text}"
            );
        }
    }
}
