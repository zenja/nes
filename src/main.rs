mod bus;
mod cartridge;
mod cpu;
mod mapper;

fn main() {
    let asm = r"
        LDX #$08
        decrement:
        DEX
        STX $0200
        CPX #$03
        BNE decrement
        STX $0201
        BRK
    ";
    let bytes = cpu::assembler::assemble(asm);
    println!("{:02X?}", bytes);
}
