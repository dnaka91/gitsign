# Signing Git commits with SSH in Rust

Minimal application that showcases how to create SSH-signed commits in Rust with the two popular crates [git2](https://github.com/rust-lang/git2-rs) ([crates.io](https://crates.io/crates/git2), [docs.rs](https://docs.rs/git2/latest/git2/)) and [gix](https://github.com/Byron/gitoxide) ([crates.io](https://crates.io/crates/gix), [docs.rs](https://docs.rs/gix/latest/gix/)).

**Note:** This sample expects an existing SSH key in your home directory `~/.ssh`. If it is encrypted it will interactively ask for the password to decrypt it for the signing step.

## Using the `gpgsig` header for SSH signatures

Although not documented anywhere, the `gpgsig` header is used for SSH signatures as well. This can be verified by making a signed commit with the Git CLI, assuming it is properly configured for SSH signing.

Then, the raw git commit can be inspected with the following command in any git repo:

```sh
git rev-list --format=raw --max-count=1 HEAD
```

## License

This project is licensed under [MIT License](LICENSE) (or <http://opensource.org/licenses/MIT>).
