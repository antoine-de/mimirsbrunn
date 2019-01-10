use git_version;

fn main() {
    git_version::set_env_with_name("CARGO_PKG_VERSION");
}
