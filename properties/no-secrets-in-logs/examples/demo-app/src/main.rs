// Demo application for examples/demo.classification.toml.
//
// It plays the role of agent-written code: it uses only the agent-facing API
// of `nosecrets` and passes the local check and the gate. It is the positive
// reference for the gate test suite.

use vstd::prelude::*;
use nosecrets::{audit, deliver, Public, Secret};
use nosecrets::crypto::{self, Digest, HashKey, PasswordHash};
use nosecrets::declassify;
use nosecrets::schema::{self, ApiKey, Command, Email, Identifier, Password};

verus! {

struct Account {
    user: Public,
    email: Secret<Email>,
    password: PasswordHash,
    key_digest: Digest<ApiKey>,
    key_last4: Public,
}

struct State {
    accounts: Vec<Account>,
    hash_key: HashKey,
    minutes: u64,
}

fn log_simple(event: &'static str, outcome: &'static str) {
    let mut e = audit::Entry::new(&Public::lit(event));
    e.field("outcome", &Public::lit(outcome));
    audit::emit(e);
}

fn respond(text: &'static str) {
    let mut line = deliver::Line::new();
    line.public(&Public::lit(text));
    deliver::emit(line);
}

fn signup(st: &mut State, user: Public, email: Secret<Email>, password: Secret<Password>) {
    if !declassify::len_between(&password, 12, 128) {
        let mut e = audit::Entry::new(&Public::lit("signup"));
        e.field("outcome", &Public::lit("failure"));
        e.field("reason", &Public::lit("invalid_input"));
        e.field("user", &user);
        audit::emit(e);
        respond("error invalid_input");
        return;
    }
    let key = schema::generate_api_key();
    let last4 = schema::last_n_api_key(&key);
    let mut e = audit::Entry::new(&Public::lit("signup"));
    e.field("outcome", &Public::lit("success"));
    e.field("user", &user);
    e.field("email_ref", &crypto::keyed_hash(&st.hash_key, &email));
    e.field("key_last4", &last4);
    audit::emit(e);
    let mut line = deliver::Line::new();
    line.public(&Public::lit("ok key"));
    line.secret(&key);
    deliver::emit(line);
    st.accounts.push(Account {
        user,
        email,
        password: crypto::password_hash(&password),
        key_digest: crypto::digest(&key),
        key_last4: last4,
    });
}

/// R7.4-style rule: the submitted identifier is logged only if it matches a
/// known user name; otherwise `<unknown>` is logged.
fn signin(st: &State, identifier: Secret<Identifier>, password: Secret<Password>) {
    let mut i: usize = 0;
    while i < st.accounts.len()
        invariant i <= st.accounts@.len(),
        decreases st.accounts@.len() - i,
    {
        let acct = &st.accounts[i];
        if declassify::eq_public(&identifier, &acct.user) {
            let shown = declassify::known_identifier(&identifier, &acct.user);
            let ok = crypto::password_verify(&password, &acct.password);
            let mut e = audit::Entry::new(&Public::lit("signin"));
            e.field("outcome", &Public::lit(if ok { "success" } else { "failure" }));
            e.field("identifier", &shown);
            audit::emit(e);
            respond(if ok { "ok" } else { "error signin_failed" });
            return;
        }
        i = i + 1;
    }
    let mut e = audit::Entry::new(&Public::lit("signin"));
    e.field("outcome", &Public::lit("failure"));
    e.field("identifier", &Public::lit("<unknown>"));
    audit::emit(e);
    respond("error signin_failed");
}

fn use_key(st: &State, key: Secret<ApiKey>) {
    if !declassify::is_hex(&key, 32) {
        log_simple("use_key", "invalid_input");
        respond("error invalid_input");
        return;
    }
    let d = crypto::digest(&key);
    let mut i: usize = 0;
    while i < st.accounts.len()
        invariant
            i <= st.accounts@.len(),
            key@.len() == 32,
        decreases st.accounts@.len() - i,
    {
        if crypto::digest_eq(&d, &st.accounts[i].key_digest) {
            let mut e = audit::Entry::new(&Public::lit("use_key"));
            e.field("outcome", &Public::lit("success"));
            e.field("user", &st.accounts[i].user);
            // The submitted key has exactly 32 characters (is_hex above), so
            // the last_n precondition holds.
            e.field("key_last4", &schema::last_n_api_key(&key));
            audit::emit(e);
            let mut line = deliver::Line::new();
            line.public(&Public::lit("ok"));
            line.public(&st.accounts[i].user);
            deliver::emit(line);
            return;
        }
        i = i + 1;
    }
    log_simple("use_key", "failure");
    respond("error unknown_key");
}

#[verifier::exec_allows_no_decreases_clause]
fn main() {
    let mut st = State { accounts: Vec::new(), hash_key: HashKey::generate(), minutes: 0 };
    loop {
        let cmd = match schema::next_command() {
            Some(c) => c,
            None => break,
        };
        match cmd {
            Command::Signup { user, email, password } => signup(&mut st, user, email, password),
            Command::Signin { identifier, password } => signin(&st, identifier, password),
            Command::UseKey { key } => use_key(&st, key),
            Command::Wait { minutes } => {
                if minutes <= u64::MAX - st.minutes {
                    st.minutes = st.minutes + minutes;
                }
                let mut e = audit::Entry::new(&Public::lit("wait"));
                e.field("now", &Public::num(st.minutes));
                audit::emit(e);
                respond("ok");
            }
            Command::Quit => {
                log_simple("quit", "success");
                break;
            }
            Command::Invalid { command } => {
                let mut e = audit::Entry::new(&Public::lit("invalid"));
                e.field("command", &command);
                audit::emit(e);
                respond("error invalid_input");
            }
            Command::Empty => {}
        }
    }
}

} // verus!
