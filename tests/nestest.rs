use cpu::cpu::Cpu;
use nes::cpu;
use std::path::PathBuf;

#[test]
fn test_nestest() {
    let mut nes_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    nes_path.push("tests/resources/nestest.nes");

    let mut cpu = Cpu::new();
    cpu.load_ines(nes_path);
    cpu.reset();
    // set PC to C000 to run nestest in automation mode
    cpu.pc = 0xC000;

    let mut nes_log_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    nes_log_path.push("tests/resources/nestest.simplified.log");

    let nes_logs: String = std::fs::read_to_string(nes_log_path).expect("Can't read nestest logs");
    let nes_log_lines: Vec<&str> = nes_logs.split("\n").collect();
    let mut line_idx = 0;
    cpu.run_with_callback(|cpu| {
        let trace_line = cpu.trace();
        assert_eq!(trace_line, nes_log_lines[line_idx]);
        line_idx += 1;
    });
}
