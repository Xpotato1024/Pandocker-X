#[test]
fn cargo_registers_the_real_cli_binary() {
    assert!(std::path::Path::new(env!("CARGO_BIN_EXE_pdx")).is_file());
}
