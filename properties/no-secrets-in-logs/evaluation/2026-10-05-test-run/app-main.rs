use vstd::prelude::*;
use nosecrets::{audit, deliver, Public, Secret};
use nosecrets::{crypto, declassify, schema};

verus! {

struct Identity {
    username: Public,
    email: Secret<schema::Email>,
    pw: crypto::PasswordHash,
    verified: bool,
    vcode: Option<crypto::Digest<schema::VerificationCode>>,
    vtime: u64,
    vattempts: u64,
    rcode: Option<crypto::Digest<schema::RecoveryCode>>,
    rtime: u64,
    rattempts: u64,
    fails: u64,
    lock_t: u64,
    epoch: u64,
}

struct Session {
    token: Secret<schema::SessionToken>,
    ident: usize,
    start: u64,
    epoch: u64,
}

struct Token {
    digest: crypto::Digest<schema::AccessToken>,
    name: Public,
    last8: Public,
    owner: usize,
}

struct VMsg {
    ident: usize,
    code: Secret<schema::VerificationCode>,
}

struct RMsg {
    ident: usize,
    code: Secret<schema::RecoveryCode>,
}

struct State {
    key: crypto::HashKey,
    now: u64,
    seq: u64,
    ids: Vec<Identity>,
    sessions: Vec<Session>,
    tokens: Vec<Token>,
    vmsgs: Vec<VMsg>,
    rmsgs: Vec<RMsg>,
}

/// Public context of one audit entry; "-" means not applicable.
struct Ctx {
    user: Public,
    eh: Public,
    ident: Public,
    name: Public,
    last8: Public,
}

impl Ctx {
    fn new() -> (r: Ctx) {
        Ctx {
            user: Public::lit("-"),
            eh: Public::lit("-"),
            ident: Public::lit("-"),
            name: Public::lit("-"),
            last8: Public::lit("-"),
        }
    }
}

impl State {
    fn new() -> (r: State) {
        State {
            key: crypto::HashKey::generate(),
            now: 0,
            seq: 1,
            ids: Vec::new(),
            sessions: Vec::new(),
            tokens: Vec::new(),
            vmsgs: Vec::new(),
            rmsgs: Vec::new(),
        }
    }
}

fn log(st: &State, event: &'static str, outcome: &'static str, reason: &'static str, c: &Ctx) {
    let mut e = audit::Entry::new(&Public::lit(event));
    e.field("seq", &Public::num(st.seq));
    e.field("time", &Public::num(st.now));
    e.field("outcome", &Public::lit(outcome));
    e.field("reason", &Public::lit(reason));
    e.field("user", &c.user);
    e.field("email_hash", &c.eh);
    e.field("identifier", &c.ident);
    e.field("token_name", &c.name);
    e.field("token_last8", &c.last8);
    audit::emit(e);
}

fn say(msg: &'static str) {
    let mut l = deliver::Line::new();
    l.public(&Public::lit(msg));
    deliver::emit(l);
}

fn valid_username(p: &Public) -> (r: bool) {
    let n = p.len();
    if n < 3 || n > 32 {
        return false;
    }
    let mut i: usize = 0;
    while i < n
        invariant
            i <= n,
            n == p@.len(),
        decreases n - i,
    {
        let c = p.char_at(i);
        if !(('a' <= c && c <= 'z') || ('0' <= c && c <= '9') || c == '-' || c == '_') {
            return false;
        }
        i = i + 1;
    }
    true
}

fn username_taken(ids: &Vec<Identity>, u: &Public) -> (r: bool) {
    let mut i: usize = 0;
    while i < ids.len()
        invariant
            i <= ids.len(),
        decreases ids.len() - i,
    {
        if ids[i].username.same(u) {
            return true;
        }
        i = i + 1;
    }
    false
}

fn find_by_email(ids: &Vec<Identity>, e: &Secret<schema::Email>) -> (r: Option<usize>)
    ensures
        match r {
            Some(i) => i < ids.len(),
            None => true,
        },
{
    let mut i: usize = 0;
    while i < ids.len()
        invariant
            i <= ids.len(),
        decreases ids.len() - i,
    {
        if declassify::eq(e, &ids[i].email) {
            return Some(i);
        }
        i = i + 1;
    }
    None
}

fn find_login(ids: &Vec<Identity>, ident: &Secret<schema::LoginIdentifier>) -> (r: Option<usize>)
    ensures
        match r {
            Some(i) => i < ids.len(),
            None => true,
        },
{
    let mut i: usize = 0;
    while i < ids.len()
        invariant
            i <= ids.len(),
        decreases ids.len() - i,
    {
        let by_name = declassify::eq_public(ident, &ids[i].username);
        let by_mail = declassify::eq_with(ident, &ids[i].email);
        if by_name || by_mail {
            return Some(i);
        }
        i = i + 1;
    }
    None
}

fn find_session(st: &State, tok: &Secret<schema::SessionToken>) -> (r: Option<(usize, usize)>)
    ensures
        match r {
            Some((s, i)) => s < st.sessions.len() && i < st.ids.len(),
            None => true,
        },
{
    let mut j: usize = 0;
    while j < st.sessions.len()
        invariant
            j <= st.sessions.len(),
        decreases st.sessions.len() - j,
    {
        if declassify::eq(tok, &st.sessions[j].token) {
            let owner = st.sessions[j].ident;
            if owner < st.ids.len() {
                let fresh = st.now >= st.sessions[j].start && st.now - st.sessions[j].start < 1440;
                if fresh && st.sessions[j].epoch == st.ids[owner].epoch {
                    return Some((j, owner));
                }
            }
        }
        j = j + 1;
    }
    None
}

fn find_token_by_name(st: &State, owner: usize, name: &Public) -> (r: Option<usize>)
    ensures
        match r {
            Some(k) => k < st.tokens.len(),
            None => true,
        },
{
    let mut k: usize = 0;
    while k < st.tokens.len()
        invariant
            k <= st.tokens.len(),
        decreases st.tokens.len() - k,
    {
        if st.tokens[k].owner == owner && st.tokens[k].name.same(name) {
            return Some(k);
        }
        k = k + 1;
    }
    None
}

fn handle_register(
    st: &mut State,
    username: Public,
    email: Secret<schema::Email>,
    password: Secret<schema::Password>,
) {
    let mut c = Ctx::new();
    c.eh = crypto::keyed_hash(&st.key, &email);
    let mut ok = true;
    let mut reason: &'static str = "none";
    if !valid_username(&username) {
        ok = false;
        reason = "invalid_input";
    } else if !declassify::len_between(&email, 3, 254) {
        ok = false;
        reason = "invalid_input";
    } else if !declassify::len_between(&password, 12, 128) {
        ok = false;
        reason = "invalid_input";
    } else if username_taken(&st.ids, &username) || find_by_email(&st.ids, &email).is_some() {
        ok = false;
        reason = "duplicate";
    }
    c.user = username.duplicate();
    if ok {
        let pw = crypto::password_hash(&password);
        let code = schema::generate_verification_code();
        let d = crypto::digest(&code);
        let idx = st.ids.len();
        let now = st.now;
        st.ids.push(Identity {
            username,
            email,
            pw,
            verified: false,
            vcode: Some(d),
            vtime: now,
            vattempts: 0,
            rcode: None,
            rtime: 0,
            rattempts: 0,
            fails: 0,
            lock_t: 0,
            epoch: 0,
        });
        st.vmsgs.push(VMsg { ident: idx, code });
        say("ok registered");
        log(st, "registration.success", "success", "none", &c);
    } else {
        say("error registration failed");
        log(st, "registration.failure", "failure", reason, &c);
    }
}

fn handle_verify(st: &mut State, email: Secret<schema::Email>, code: Secret<schema::VerificationCode>) {
    let mut c = Ctx::new();
    c.eh = crypto::keyed_hash(&st.key, &email);
    let mut ok = false;
    let mut reason: &'static str = "unknown_email";
    if !declassify::is_digits(&code, 6) {
        reason = "invalid_input";
    } else {
        match find_by_email(&st.ids, &email) {
            None => {}
            Some(i) => {
                let now = st.now;
                let nd = crypto::digest(&code);
                let mut id = st.ids.remove(i);
                c.user = id.username.duplicate();
                let have = id.vcode.is_some();
                let matches = match &id.vcode {
                    Some(d) => crypto::digest_eq(d, &nd),
                    None => false,
                };
                let fresh = now >= id.vtime && now - id.vtime <= 15;
                if id.verified {
                    reason = "already_verified";
                } else if !have {
                    reason = "no_code";
                } else if !fresh {
                    reason = "code_expired";
                    id.vcode = None;
                } else if matches {
                    ok = true;
                    reason = "none";
                    id.verified = true;
                    id.vcode = None;
                } else if id.vattempts < 4 {
                    reason = "wrong_code";
                    id.vattempts = id.vattempts + 1;
                } else {
                    reason = "too_many_attempts";
                    id.vattempts = 0;
                    id.vcode = None;
                }
                st.ids.insert(i, id);
            }
        }
    }
    if ok {
        say("ok verified");
        log(st, "verification.success", "success", "none", &c);
    } else {
        say("error verification failed");
        log(st, "verification.failure", "failure", reason, &c);
    }
}

fn handle_resend(st: &mut State, email: Secret<schema::Email>) {
    let mut c = Ctx::new();
    c.eh = crypto::keyed_hash(&st.key, &email);
    let mut reason: &'static str = "unknown_email";
    let mut ok = false;
    match find_by_email(&st.ids, &email) {
        None => {}
        Some(i) => {
            let now = st.now;
            let mut id = st.ids.remove(i);
            c.user = id.username.duplicate();
            if id.verified {
                reason = "already_verified";
                st.ids.insert(i, id);
            } else {
                let code = schema::generate_verification_code();
                id.vcode = Some(crypto::digest(&code));
                id.vtime = now;
                id.vattempts = 0;
                st.ids.insert(i, id);
                st.vmsgs.push(VMsg { ident: i, code });
                ok = true;
                reason = "none";
            }
        }
    }
    say("ok");
    if ok {
        log(st, "verification.resent", "success", reason, &c);
    } else {
        log(st, "verification.resent", "ignored", reason, &c);
    }
}

fn handle_login(st: &mut State, ident: Secret<schema::LoginIdentifier>, password: Secret<schema::Password>) {
    let mut c = Ctx::new();
    c.ident = Public::lit("<unknown>");
    let mut ok = false;
    let mut reason: &'static str = "unknown_identifier";
    match find_login(&st.ids, &ident) {
        None => {}
        Some(i) => {
            if declassify::eq_public(&ident, &st.ids[i].username) {
                c.ident = declassify::known_identifier(&ident, &st.ids[i].username);
            } else {
                c.ident = Public::lit("<email>");
            }
            c.user = st.ids[i].username.duplicate();
            let now = st.now;
            let mut id = st.ids.remove(i);
            let locked = id.fails >= 5 && now >= id.lock_t && now - id.lock_t < 15;
            if id.fails >= 5 && !locked {
                id.fails = 0;
            }
            if locked {
                reason = "locked";
            } else if crypto::password_verify(&password, &id.pw) {
                ok = true;
                reason = "none";
                id.fails = 0;
                let tok = schema::generate_session_token();
                let mut l = deliver::Line::new();
                l.secret(&tok);
                deliver::emit(l);
                st.sessions.push(Session { token: tok, ident: i, start: now, epoch: id.epoch });
            } else {
                reason = "wrong_password";
                if id.fails < 1000 {
                    id.fails = id.fails + 1;
                }
                if id.fails >= 5 {
                    id.lock_t = now;
                }
            }
            st.ids.insert(i, id);
        }
    }
    if ok {
        log(st, "login.success", "success", "none", &c);
    } else {
        say("error login failed");
        log(st, "login.failure", "failure", reason, &c);
    }
}

fn handle_whoami(st: &mut State, tok: Secret<schema::SessionToken>) {
    let mut c = Ctx::new();
    match find_session(st, &tok) {
        None => {
            say("error invalid session");
            log(st, "session.whoami", "failure", "invalid_session", &c);
        }
        Some((_s, i)) => {
            c.user = st.ids[i].username.duplicate();
            let mut l = deliver::Line::new();
            l.public(&st.ids[i].username);
            l.secret(&st.ids[i].email);
            if st.ids[i].verified {
                l.public(&Public::lit("verified"));
            } else {
                l.public(&Public::lit("unverified"));
            }
            deliver::emit(l);
            log(st, "session.whoami", "success", "none", &c);
        }
    }
}

fn handle_logout(st: &mut State, tok: Secret<schema::SessionToken>) {
    let mut c = Ctx::new();
    match find_session(st, &tok) {
        None => {
            say("error invalid session");
            log(st, "session.logout", "failure", "invalid_session", &c);
        }
        Some((s, i)) => {
            c.user = st.ids[i].username.duplicate();
            let _ = st.sessions.remove(s);
            say("ok logged out");
            log(st, "session.logout", "success", "none", &c);
        }
    }
}

fn handle_recover(st: &mut State, email: Secret<schema::Email>) {
    let mut c = Ctx::new();
    c.eh = crypto::keyed_hash(&st.key, &email);
    let mut found = false;
    match find_by_email(&st.ids, &email) {
        None => {}
        Some(i) => {
            found = true;
            let now = st.now;
            let mut id = st.ids.remove(i);
            c.user = id.username.duplicate();
            let code = schema::generate_recovery_code();
            id.rcode = Some(crypto::digest(&code));
            id.rtime = now;
            id.rattempts = 0;
            st.ids.insert(i, id);
            st.rmsgs.push(RMsg { ident: i, code });
        }
    }
    say("ok if the account exists, a recovery code was sent");
    if found {
        log(st, "recovery.requested", "success", "none", &c);
    } else {
        log(st, "recovery.requested", "ignored", "unknown_email", &c);
    }
}

fn handle_reset(
    st: &mut State,
    email: Secret<schema::Email>,
    code: Secret<schema::RecoveryCode>,
    new_password: Secret<schema::Password>,
) {
    let mut c = Ctx::new();
    c.eh = crypto::keyed_hash(&st.key, &email);
    let mut ok = false;
    let mut reason: &'static str = "unknown_email";
    if !declassify::is_digits(&code, 6) || !declassify::len_between(&new_password, 12, 128) {
        reason = "invalid_input";
    } else {
        match find_by_email(&st.ids, &email) {
            None => {}
            Some(i) => {
                let now = st.now;
                let nd = crypto::digest(&code);
                let mut id = st.ids.remove(i);
                c.user = id.username.duplicate();
                let have = id.rcode.is_some();
                let matches = match &id.rcode {
                    Some(d) => crypto::digest_eq(d, &nd),
                    None => false,
                };
                let fresh = now >= id.rtime && now - id.rtime <= 15;
                if !have {
                    reason = "no_code";
                } else if !fresh {
                    reason = "code_expired";
                    id.rcode = None;
                } else if matches {
                    ok = true;
                    reason = "none";
                    id.pw = crypto::password_hash(&new_password);
                    id.rcode = None;
                    id.fails = 0;
                    if id.epoch < u64::MAX {
                        id.epoch = id.epoch + 1;
                    }
                } else if id.rattempts < 4 {
                    reason = "wrong_code";
                    id.rattempts = id.rattempts + 1;
                } else {
                    reason = "too_many_attempts";
                    id.rattempts = 0;
                    id.rcode = None;
                }
                st.ids.insert(i, id);
            }
        }
    }
    if ok {
        say("ok password reset");
        log(st, "recovery.success", "success", "none", &c);
    } else {
        say("error reset failed");
        log(st, "recovery.failure", "failure", reason, &c);
    }
}

fn handle_token_create(st: &mut State, tok: Secret<schema::SessionToken>, name: Public) {
    let mut c = Ctx::new();
    c.name = name.duplicate();
    match find_session(st, &tok) {
        None => {
            say("error invalid session");
            log(st, "token.create", "failure", "invalid_session", &c);
        }
        Some((_s, i)) => {
            c.user = st.ids[i].username.duplicate();
            if !st.ids[i].verified {
                say("error email not verified");
                log(st, "token.create", "failure", "unverified", &c);
            } else if name.len() < 1 || name.len() > 64 {
                say("error invalid name");
                log(st, "token.create", "failure", "invalid_input", &c);
            } else if find_token_by_name(st, i, &name).is_some() {
                say("error name in use");
                log(st, "token.create", "failure", "duplicate", &c);
            } else {
                let t = schema::generate_access_token();
                let last8 = schema::last_n_access_token(&t);
                c.last8 = last8.duplicate();
                let mut l = deliver::Line::new();
                l.secret(&t);
                deliver::emit(l);
                st.tokens.push(Token { digest: crypto::digest(&t), name, last8, owner: i });
                log(st, "token.created", "success", "none", &c);
            }
        }
    }
}

fn handle_token_list(st: &mut State, tok: Secret<schema::SessionToken>) {
    let mut c = Ctx::new();
    match find_session(st, &tok) {
        None => {
            say("error invalid session");
            log(st, "token.list", "failure", "invalid_session", &c);
        }
        Some((_s, i)) => {
            c.user = st.ids[i].username.duplicate();
            let mut l = deliver::Line::new();
            l.public(&Public::lit("tokens"));
            let mut k: usize = 0;
            while k < st.tokens.len()
                invariant
                    k <= st.tokens.len(),
                decreases st.tokens.len() - k,
            {
                if st.tokens[k].owner == i {
                    l.public(&st.tokens[k].name);
                    l.public(&st.tokens[k].last8);
                }
                k = k + 1;
            }
            deliver::emit(l);
            log(st, "token.list", "success", "none", &c);
        }
    }
}

fn handle_token_delete(st: &mut State, tok: Secret<schema::SessionToken>, name: Public) {
    let mut c = Ctx::new();
    c.name = name.duplicate();
    match find_session(st, &tok) {
        None => {
            say("error invalid session");
            log(st, "token.delete", "failure", "invalid_session", &c);
        }
        Some((_s, i)) => {
            c.user = st.ids[i].username.duplicate();
            match find_token_by_name(st, i, &name) {
                None => {
                    say("error no such token");
                    log(st, "token.delete", "failure", "not_found", &c);
                }
                Some(k) => {
                    c.last8 = st.tokens[k].last8.duplicate();
                    let _ = st.tokens.remove(k);
                    say("ok deleted");
                    log(st, "token.deleted", "success", "none", &c);
                }
            }
        }
    }
}

fn handle_token_check(st: &mut State, token: Secret<schema::AccessToken>) {
    let mut c = Ctx::new();
    if !declassify::is_hex(&token, 40) {
        say("error invalid token");
        log(st, "token.check", "failure", "invalid_input", &c);
        return;
    }
    let nd = crypto::digest(&token);
    let mut k: usize = 0;
    while k < st.tokens.len()
        invariant
            k <= st.tokens.len(),
        decreases st.tokens.len() - k,
    {
        if crypto::digest_eq(&st.tokens[k].digest, &nd) {
            let owner = st.tokens[k].owner;
            if owner < st.ids.len() {
                c.user = st.ids[owner].username.duplicate();
                c.name = st.tokens[k].name.duplicate();
                c.last8 = st.tokens[k].last8.duplicate();
                let mut l = deliver::Line::new();
                l.public(&st.ids[owner].username);
                deliver::emit(l);
                log(st, "token.check", "success", "none", &c);
                return;
            }
        }
        k = k + 1;
    }
    say("error invalid token");
    log(st, "token.check", "failure", "unknown_token", &c);
}

fn handle_outbox(st: &mut State, email: Secret<schema::Email>) {
    let mut c = Ctx::new();
    c.eh = crypto::keyed_hash(&st.key, &email);
    let mut l = deliver::Line::new();
    l.public(&Public::lit("outbox"));
    match find_by_email(&st.ids, &email) {
        None => {}
        Some(i) => {
            c.user = st.ids[i].username.duplicate();
            let mut j: usize = 0;
            while j < st.vmsgs.len()
                invariant
                    j <= st.vmsgs.len(),
                decreases st.vmsgs.len() - j,
            {
                if st.vmsgs[j].ident == i {
                    l.public(&Public::lit("verification"));
                    l.secret(&st.vmsgs[j].code);
                }
                j = j + 1;
            }
            let mut m: usize = 0;
            while m < st.rmsgs.len()
                invariant
                    m <= st.rmsgs.len(),
                decreases st.rmsgs.len() - m,
            {
                if st.rmsgs[m].ident == i {
                    l.public(&Public::lit("recovery"));
                    l.secret(&st.rmsgs[m].code);
                }
                m = m + 1;
            }
        }
    }
    deliver::emit(l);
    log(st, "outbox.read", "success", "none", &c);
}

fn handle_tick(st: &mut State, minutes: u64) {
    let c = Ctx::new();
    if minutes <= u64::MAX - st.now {
        st.now = st.now + minutes;
    } else {
        st.now = u64::MAX;
    }
    say("ok");
    log(st, "clock.tick", "success", "none", &c);
}

fn dispatch(st: &mut State, cmd: schema::Command) {
    match cmd {
        schema::Command::Register { username, email, password } => {
            handle_register(st, username, email, password)
        }
        schema::Command::Verify { email, code } => handle_verify(st, email, code),
        schema::Command::ResendVerification { email } => handle_resend(st, email),
        schema::Command::Login { identifier, password } => handle_login(st, identifier, password),
        schema::Command::Whoami { session_token } => handle_whoami(st, session_token),
        schema::Command::Logout { session_token } => handle_logout(st, session_token),
        schema::Command::Recover { email } => handle_recover(st, email),
        schema::Command::ResetPassword { email, code, new_password } => {
            handle_reset(st, email, code, new_password)
        }
        schema::Command::TokenCreate { session_token, name } => {
            handle_token_create(st, session_token, name)
        }
        schema::Command::TokenList { session_token } => handle_token_list(st, session_token),
        schema::Command::TokenDelete { session_token, name } => {
            handle_token_delete(st, session_token, name)
        }
        schema::Command::TokenCheck { token } => handle_token_check(st, token),
        schema::Command::Outbox { email } => handle_outbox(st, email),
        schema::Command::Tick { minutes } => handle_tick(st, minutes),
        schema::Command::Invalid { command } => {
            let mut c = Ctx::new();
            c.name = command;
            say("error invalid command");
            log(st, "command.invalid", "failure", "invalid_input", &c);
        }
        schema::Command::Empty => {
            let c = Ctx::new();
            say("error empty command");
            log(st, "command.empty", "failure", "invalid_input", &c);
        }
    }
    if st.seq < u64::MAX {
        st.seq = st.seq + 1;
    }
}

#[verifier::exec_allows_no_decreases_clause]
fn run(st: &mut State) {
    loop {
        match schema::next_command() {
            None => {
                break;
            }
            Some(cmd) => {
                dispatch(st, cmd);
            }
        }
    }
}

fn main() {
    let mut st = State::new();
    run(&mut st);
}

} // verus!
