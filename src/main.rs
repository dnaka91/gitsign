use std::{env, fs};

use anyhow::{Context, Result};
use ssh_key::{HashAlg, LineEnding, PrivateKey};

fn main() -> Result<()> {
    let key = load_key()?;

    with_git2(&key)?;
    println!("created with GIT2 at: ./tmp-git2");

    with_gix(&key)?;
    println!("created with GIX at: ./tmp-gix");

    Ok(())
}

/// Use the `git2` crate, a `libgit2` wrapper, to initialize a new repo and create an initial commit
/// signed with the user's SSH key.
fn with_git2(key: &PrivateKey) -> Result<()> {
    use git2::{Repository, Signature};

    let dir = env::current_dir()?.join("tmp-git2");
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir)?;

    let repo = Repository::init(&dir)?;

    let mut index = repo.index()?;
    let tree = index.write_tree()?;
    let tree = repo.find_tree(tree)?;

    let author = Signature::now("Bob", "bob@example.com")?;

    let content = repo.commit_create_buffer(&author, &author, "Initial commit", &tree, &[])?;
    let content = content.as_str().context("invalid UTF-8")?;

    let sig = key
        .sign("git", HashAlg::Sha256, content.as_bytes())?
        .to_pem(LineEnding::LF)?;

    let commit = repo.commit_signed(content, sig.trim(), None)?;
    let commit = repo.find_commit(commit)?;

    repo.branch("main", &commit, true)?;

    Ok(())
}

/// Use the `gix` crate, a native Rust Git implementation, to initialize a new repo and create an
/// initial commit signed with the user's SSH key.
fn with_gix(key: &PrivateKey) -> Result<()> {
    use gix::{
        actor::SignatureRef,
        objs::{Commit, Tree, WriteTo},
        reference::log,
        refs::{
            transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
            Target,
        },
    };

    let dir = env::current_dir()?.join("tmp-gix");
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir)?;

    let repo = gix::init(dir)?;
    let tree = Tree::empty();
    let tree = repo.write_object(&tree)?.detach();

    // All this is extracted from the `Repository::commit` convenience function, which sadly doesn't
    // have a variant to allow signing before the commit, like `git2` has.
    let author = SignatureRef {
        name: "Bob".into(),
        email: "bob@example.com".into(),
        time: gix::date::Time::now_local_or_utc(),
    };

    let mut commit = Commit {
        message: "Initial commit".into(),
        tree,
        author: author.into(),
        committer: author.into(),
        encoding: None,
        parents: Default::default(),
        extra_headers: Vec::with_capacity(1),
    };

    let sig = {
        let mut msg = Vec::new();
        commit.write_to(&mut msg)?;

        key.sign("git", HashAlg::Sha256, &msg)?
            .to_pem(LineEnding::LF)?
    };

    commit
        .extra_headers
        .push(("gpgsig".into(), sig.trim().into()));

    let commit_id = repo.write_object(&commit)?;

    repo.edit_reference(RefEdit {
        change: Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: false,
                message: log::message("commit", commit.message.as_ref(), commit.parents.len()),
            },
            expected: PreviousValue::MustNotExist,
            new: Target::Peeled(commit_id.detach()),
        },
        name: "HEAD".try_into()?,
        deref: true,
    })?;

    Ok(())
}

/// Load the main SSH key.
///
/// Tries the default key locations to find some SSH key used by the user. Those are:
///
/// - `~/.ssh/id_ed25519` for a EdDSA (_Edwards-curve Digital Signature Algorithm_) key with
///   _Curve25519_.
/// - `~/.ssh/id_ecdsa` for a ECDSA (_Elliptic Curve Digital Signature Algorithm_) key.
/// - `~/.ssh/id_rsa` for a RSA (_Rivest–Shamir–Adleman_) key.
fn load_key() -> Result<PrivateKey> {
    let ssh_dir = dirs::home_dir()
        .context("failed locating home dir")?
        .join(".ssh");

    let key = ["id_ed25519", "id_ecdsa", "id_rsa"]
        .into_iter()
        .flat_map(|keyfile| fs::read(ssh_dir.join(keyfile)))
        .next()
        .context("not suitable SSH key found")?;

    let key = PrivateKey::from_openssh(key)?;

    if key.is_encrypted() {
        decrypt(key)
    } else {
        Ok(key)
    }
}

/// Ask for a password and try to decrypt the key.
///
/// This will re-ask for a password in case the key couldn't be decrypted or the user cancels the
/// whole application with _CTRL-C_.
fn decrypt(key: PrivateKey) -> Result<PrivateKey> {
    use inquire::{Password, PasswordDisplayMode};

    loop {
        let password = Password::new("SSH key password:")
            .without_confirmation()
            .with_display_mode(PasswordDisplayMode::Masked)
            .prompt()?;

        match key.decrypt(&password) {
            Ok(key) => break Ok(key),
            Err(_) => {
                eprintln!("wrong password");
                continue;
            }
        }
    }
}
