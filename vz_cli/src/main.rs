//! `vz`: the book kit's command. Everything is in the library, which the Python console script runs too.

fn main() -> vc_ui::Exit {
    vc_ui::restore_sigpipe();
    // answers `COMPLETE=bash vz` (zsh, fish) and exits; otherwise returns
    vc_complete::complete(vz_cli::command);
    vz_cli::main(std::env::args_os())
}
