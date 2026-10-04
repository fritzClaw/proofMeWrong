// Skeleton for gate test cases (tests/run_suite.py). The suite fills in the
// ITEMS, CASE and OUTSIDE placeholders below with the code of one case.
use vstd::prelude::*;
use nosecrets::{audit, deliver, Public, Secret};
use nosecrets::crypto::{self, HashKey};
use nosecrets::declassify;
use nosecrets::schema::{self, ApiKey, Command, Email, Identifier, Password, Pin};

verus! {

/*ITEMS*/

#[allow(unused_variables, unused_mut)]
fn case(hk: &HashKey, user: Public, email: Secret<Email>, password: Secret<Password>,
        identifier: Secret<Identifier>, key: Secret<ApiKey>, pin: Secret<Pin>) {
    let mut e = audit::Entry::new(&Public::lit("probe"));
    /*CASE*/
    audit::emit(e);
}

#[verifier::exec_allows_no_decreases_clause]
fn main() {
    let hk = HashKey::generate();
    loop {
        match schema::next_command() {
            Some(Command::Probe { user, email, password, identifier, key, pin }) =>
                case(&hk, user, email, password, identifier, key, pin),
            Some(_) => {}
            None => break,
        }
    }
}

} // verus!
/*OUTSIDE*/
