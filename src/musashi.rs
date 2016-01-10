// Integration with Musashi
extern crate libc;


// Register enum copied from Musashi's m68k_register_t enum
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum Register {
	/* Real registers */
	D0,		/* Data registers */
	D1,
	D2,
	D3,
	D4,
	D5,
	D6,
	D7,
	A0,		/* Address registers */
	A1,
	A2,
	A3,
	A4,
	A5,
	A6,
	A7,
	PC,		/* Program Counter */
	SR,		/* Status Register */
	SP,		/* The current Stack Pointer (located in A7) */
	USP,		/* User Stack Pointer */
	ISP,		/* Interrupt Stack Pointer */
	MSP,		/* Master Stack Pointer */
	SFC,		/* Source Function Code */
	DFC,		/* Destination Function Code */
	VBR,		/* Vector Base Register */
	CACR,		/* Cache Control Register */
	CAAR,		/* Cache Address Register */

	/* Assumed registers */
	/* These are cheat registers which emulate the 1-longword prefetch
	 * present in the 68000 and 68010.
	 */
	PrefAddr,	/* Last prefetch address */
	PrefData,	/* Last prefetch data */

	/* Convenience registers */
	PPC,		/* Previous value in the program counter */
	IR,			/* Instruction register */
	CpuType	/* Type of CPU being run */
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
enum CpuType
{
	Invalid,
	M68000,
	M68010,
	M68EC020,
	M68020,
	M68030,		/* Supported by disassembler ONLY */
	M68040		/* Supported by disassembler ONLY */
}
#[link(name = "musashi", kind = "static")]
extern {
	fn m68k_init();
	fn m68k_set_cpu_type(cputype: CpuType);
	fn m68k_pulse_reset();
	fn m68k_execute(num_cycles: i32) -> i32;
	fn m68k_get_reg(context: *mut libc::c_void, regnum: Register) -> u32;
	fn m68k_set_reg(regnum: Register, value: u32);
}
use ram::{Operation, AddressBus, AddressSpace, SUPERVISOR_PROGRAM, SUPERVISOR_DATA, USER_PROGRAM, USER_DATA, ADDRBUS_MASK};
static mut musashi_memory:  [u8; 16*1024*1024+16] = [0xaa; 16*1024*1024+16];
// as statics are not allowed to have destructors, allocate a
// big enough array to hold the small number of operations
// expected from executing a very limited number of opcodes
static mut musashi_ops: [Operation; 128] = [Operation::None; 128];
static mut musashi_opcount: usize = 0;
static mut musashi_address_space: AddressSpace = SUPERVISOR_PROGRAM;

unsafe fn register_op(op: Operation) {
	if musashi_opcount < musashi_ops.len() {
		// println!("mem_op {:?}", op);
		musashi_ops[musashi_opcount] = op;
		musashi_opcount += 1;
	}
}
// callbacks from Musashi
#[no_mangle]
pub extern fn cpu_read_byte(address: u32) -> u32 {
	unsafe {
		let address = address & ADDRBUS_MASK;
		let addr = address as usize;
		let value = musashi_memory[addr];
		let op = Operation::ReadByte(musashi_address_space, address, value);
		register_op(op);
		value as u32
	}
}
#[no_mangle]
pub extern fn cpu_read_word(address: u32) -> u32 {
	unsafe {
		let address = address & ADDRBUS_MASK;
		let addr = address as usize;
		let value =  (musashi_memory[addr+0] as u16) << 8
					|(musashi_memory[addr+1] as u16) << 0;
		let op = Operation::ReadWord(musashi_address_space, address, value);
		register_op(op);
		value as u32
	}
}
#[no_mangle]
pub extern fn cpu_read_long(address: u32) -> u32 {
	unsafe {
		let addr = (address & ADDRBUS_MASK) as usize;
		let value = ((musashi_memory[addr+0] as u32) << 24
					|(musashi_memory[addr+1] as u32) << 16
					|(musashi_memory[addr+2] as u32) <<  8
					|(musashi_memory[addr+3] as u32) <<  0) as u32;
		let op = Operation::ReadLong(musashi_address_space, address, value);
		register_op(op);
		value
	}
}

#[no_mangle]
pub extern fn cpu_write_byte(address: u32, value: u32) {
	unsafe {
		let op = Operation::WriteByte(musashi_address_space, address, value);
		let address = (address & ADDRBUS_MASK) as usize;
		register_op(op);
		musashi_memory[address+0] = (value & 0xff) as u8;
	}
}
#[no_mangle]
pub extern fn cpu_write_word(address: u32, value: u32) {
	unsafe {
		let op = Operation::WriteWord(musashi_address_space, address, value);
		let address = (address & ADDRBUS_MASK) as usize;
		register_op(op);
		musashi_memory[address+0] = ((value & 0xff00) >> 8) as u8;
		musashi_memory[address+1] = ((value & 0x00ff) >> 0) as u8;
	}
}
#[no_mangle]
pub extern fn cpu_write_long(address: u32, value: u32) {
	unsafe {
		let op = Operation::WriteLong(musashi_address_space, address, value);
		let address = (address & ADDRBUS_MASK) as usize;
		register_op(op);
		musashi_memory[address+0] = ((value & 0xff000000) >> 24) as u8;
		musashi_memory[address+1] = ((value & 0x00ff0000) >> 16) as u8;
		musashi_memory[address+2] = ((value & 0x0000ff00) >>  8) as u8;
		musashi_memory[address+3] = ((value & 0x000000ff) >>  0) as u8;
	}
}

#[no_mangle]
pub extern fn cpu_pulse_reset() {panic!("pr")}
#[no_mangle]
pub extern fn cpu_long_branch() {}
#[no_mangle]
pub extern fn cpu_set_fc(fc: u32) {
	unsafe {
		musashi_address_space = match fc {
			1 => USER_DATA,
			2 => USER_PROGRAM,
			5 => SUPERVISOR_DATA,
			6 => SUPERVISOR_PROGRAM,
			_ => panic!("unknown fc: {}", fc),
		};
		// println!("set_fc {:?}", musashi_address_space);
	}
}
#[allow(unused_variables)]
#[no_mangle]
pub extern fn cpu_irq_ack(level: i32) -> i32 {panic!("ia")}
#[no_mangle]
pub extern fn cpu_instr_callback() {}

use std::ptr;

#[allow(unused_variables)]
pub fn experimental_communication() {
	let mutex = MUSASHI_LOCK.lock().unwrap();
	unsafe {
		m68k_init();
		m68k_set_cpu_type(CpuType::M68000);
		m68k_set_reg(Register::D0, 123);
		println!("D0: {}", m68k_get_reg(ptr::null_mut(), Register::D0));
	}
}

#[allow(unused_variables)]
pub fn roundtrip_register(reg: Register, value: u32) -> u32 {
	let mutex = MUSASHI_LOCK.lock().unwrap();
	unsafe {
		m68k_init();
		m68k_set_cpu_type(CpuType::M68000);
		m68k_set_reg(reg, value);
		m68k_get_reg(ptr::null_mut(), reg)
	}
}

use cpu::{Core, Cycles};

static REGS:[Register; 16] = [Register::D0, Register::D1, Register::D2, Register::D3, Register::D4, Register::D5, Register::D6, Register::D7, Register::A0, Register::A1, Register::A2, Register::A3, Register::A4, Register::A5, Register::A6, Register::A7];

fn get_ops() -> Vec<Operation> {
	let mut res: Vec<Operation> = vec![];
	unsafe {
		for i in 0..musashi_opcount {
			res.push(musashi_ops[i]);
		}
	}
	res
}

// since we know exactly where writes have occurred, undoing is much
// less work than simply rewriting all 16M
fn undo_musashi_writes() {
	for op in get_ops()
	{
		match op {
			Operation::WriteByte(_, addr, _) => cpu_write_byte(addr, 0xaa),
			Operation::WriteWord(_, addr, _) => cpu_write_word(addr, 0xaaaa),
			Operation::WriteLong(_, addr, _) => cpu_write_long(addr, 0xaaaaaaaa),
			_ => (),
		}
	}
}

pub fn initialize_musashi(core: &mut Core) {
	// println!("initialize_musashi {:?}", thread::current());
	unsafe {
		undo_musashi_writes();
		m68k_init();
		m68k_set_cpu_type(CpuType::M68000);
		cpu_write_long(0, core.ssp());
		cpu_write_long(4, core.pc);
		m68k_pulse_reset();
		// Resetting opcount, because m68k_pulse_reset causes irrelevant
		// reads from 0x00000000 to set PC/SP, a jump to PC and
		// resetting of state. But we don't want to test those ops.
		musashi_opcount = 0;
		//m68k_set_reg(Register::PC, core.pc);
		m68k_set_reg(Register::USP, core.usp());
		// if SR clears S_FLAG then SSP <- A7, A7 <- USP
		m68k_set_reg(Register::SR, core.status_register());
		for (i, &reg) in REGS.iter().enumerate() { 
			if i != 15 {
				m68k_set_reg(reg, core.dar[i]); 
			}
		}
		// just reset first and last KB of memory, as it takes too long to
		// reset all 16MB
		let last_kb = (1 << 24) - 1024;
		for i in 0..1024usize {
			musashi_memory[i] = core.mem.read_byte(SUPERVISOR_PROGRAM, i as u32) as u8;
			musashi_memory[last_kb + i] = core.mem.read_byte(SUPERVISOR_PROGRAM, (last_kb + i) as u32) as u8;
		}
	}
}

pub fn execute1(core: &mut Core) -> Cycles {
	// println!("execute1 mushashi {:?}", thread::current());
	unsafe {
		let cycle_count = m68k_execute(1);

		for (i, &reg) in REGS.iter().enumerate() {
			core.dar[i] = m68k_get_reg(ptr::null_mut(), reg);
		}
		core.pc = m68k_get_reg(ptr::null_mut(), Register::PC);
		core.inactive_usp = m68k_get_reg(ptr::null_mut(), Register::USP);
		core.sr_to_flags(m68k_get_reg(ptr::null_mut(), Register::SR));
		Cycles(cycle_count)
	}
}

#[allow(unused_variables)]
pub fn reset_and_execute1(core: &mut Core) -> Cycles {
	let mutex = MUSASHI_LOCK.lock().unwrap();
	initialize_musashi(core);
	execute1(core)
}


// Talking to Musashi isn't thread-safe, and the tests are running
// threaded, which cause intermittent test failures unless serializing
// access using something like a mutex. Musashi functions are called in
// global/static context, and statics are not allowed to have
// destructors
use std::sync::{Arc, Mutex};
// using lazy_static! to work-around "statics are not allowed to have destructors [E0493]""
lazy_static! {
	static ref MUSASHI_LOCK: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
	static ref QUICKCHECK_LOCK: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
}

#[cfg(test)]
mod tests {
	use super::*;
	use ram::SUPERVISOR_PROGRAM;
	use super::MUSASHI_LOCK;
	use super::QUICKCHECK_LOCK;
	use ram::{Operation, AddressBus};
	use cpu::Core;

	extern crate quickcheck;
	use self::quickcheck::*;
	#[derive(Copy, Clone, Debug, PartialEq)]
	struct Bitpattern(u32);
	impl Arbitrary for Bitpattern {
		fn arbitrary<G: Gen>(g: &mut G) -> Bitpattern {
			// let m : u32 = Arbitrary::arbitrary(g);
			// let mut mask: u32 = 0xF; //((m & 0xF) | (m >> 4) & 0xF) as u32;
			// let mut i : u32 = Arbitrary::arbitrary(g);
			// let mut sum: u32 = 0;
			// println!("{}/{} when {}", i, mask, g.size());
			// // 0b11001100 => 0xFF00FF00
			// while i > 0 {
			// 	sum += if i & 1 == 1 { mask } else { 0 };
			// 	i >>= 1;
			// 	mask <<= 4;
			// }

			// when size 256, could generate any 32 bit pattern
			// let i1: u32 = Arbitrary::arbitrary(g);
			// let i2: u32 = Arbitrary::arbitrary(g);
			// let i3: u32 = Arbitrary::arbitrary(g);
			// let i4: u32 = Arbitrary::arbitrary(g);
			// let sum: u32 = (i1 << 24) | (i2 << 16) | (i3 << 8) | i4;
			// println!("{:b} when {}", i4, g.size());
			Bitpattern(Arbitrary::arbitrary(g))
		}
		fn shrink(&self) -> Box<Iterator<Item=Self>> {
			match *self {
				Bitpattern(x) => {
					let xs = x.shrink(); // should shrink Bitpattern by clearing bits, not setting new ones
					let tagged = xs //.inspect(|x|println!("{}", x))
					.map(Bitpattern);
					Box::new(tagged)
				}
			}
		}
	}

	impl Arbitrary for Register {
		fn arbitrary<G: Gen>(g: &mut G) -> Register {
			let regs = [Register::D0, Register::D1, Register::D2, Register::D3, Register::D4, Register::D5, Register::D6, Register::D7, Register::A0, Register::A1, Register::A2, Register::A3, Register::A4, Register::A5, Register::A6, 
			Register::SR, // Register::A7, Register::SP, Register::PC
			];
			//println!("{}",i);
			if let Some(&reg) = g.choose(&regs) {
				reg
			} else {
				unreachable!();
			}
		}
	}

	extern crate rand;

	use itertools::{Itertools, assert_equal};
	use cpu::ops::*;
	use super::get_ops;
	// struct OpSeq {
	// 	mask: u32,
	// 	matching: u32,
	// 	current_op: u32,
	// }
	// impl OpSeq {
	// 	fn new(mask: u32, matching: u32) -> OpSeq {
	// 		OpSeq { mask: mask, matching: matching, current_op: 0 }
	// 	}
	// }
	// impl Iterator for OpSeq {
	// 	type Item = u32;
	// 	fn next(&mut self) -> Option<u32> {
	// 		if self.current_op == 0x10000 {
	// 			None
	// 		} else {
	// 			while (self.current_op & self.mask) != self.matching && self.current_op < 0x10000 {
	// 				self.current_op += 1;
	// 			}
	// 			if self.current_op == 0x10000 {
	// 				return None;
	// 			}
	// 			let res = Some(self.current_op);
	// 			self.current_op += 1;
	// 			res
	// 		}
	// 	}
	// }

	fn opcodes(mask: u32, matching: u32) -> Vec<u16> {
		(matching..0x10000u32)
			.filter(|opcode| (opcode & mask) == matching)
			.map(|v|v as u16).collect::<Vec<u16>>()
	}
	macro_rules! opcodes {
	  ($mask:expr , $matching:expr) => {(0..0x10000).filter(|opcode| (opcode & $mask) == $matching)}
	}

	#[test]
	fn opcodes_from_mask_and_matching(){
		let mut opseq = Vec::new();
		opseq.extend(opcodes!(MASK_OUT_X_Y, OP_ABCD_8_RR));
		assert_eq!(64, opseq.len());
		let ops = opseq.iter().unique();
		assert_eq!(64, ops.count());
		if let Some(&min) = opseq.iter().min() {
			assert_eq!(0b1100000100000000, min);
		}
		if let Some(&max) = opseq.iter().max() {
			assert_eq!(0b1100111100000111, max);
		}
		for code in opseq.iter() {
			assert_eq!(OP_ABCD_8_RR, code & OP_ABCD_8_RR);
		}
	}

	static mut opcode_under_test: u16 = 0;

	fn hammer_cores_even_addresses(rs: Vec<(Register, Bitpattern)>) -> bool {
		let mem_mask = (2<<24)-2; // keep even
		hammer_cores_with(mem_mask, rs)
	}
	fn hammer_cores(rs: Vec<(Register, Bitpattern)>) -> bool {
		let mem_mask = (2<<24)-1; // allow odd
		hammer_cores_with(mem_mask, rs)
	}

	fn hammer_cores_with(mem_mask: u32, rs: Vec<(Register, Bitpattern)>) -> bool {
		let pc = 0x40;
		let mem = unsafe {
			[((opcode_under_test >> 8) & 0xff) as u8, (opcode_under_test & 0xff) as u8]
		};
		let mut musashi = Core::new_mem(pc, &mem);
		const STACK_MASK:u32 = (1024-16); // keep even
		musashi.inactive_ssp = 0x128;
		musashi.inactive_usp = 0x128;
		for r in 0..8 {
			musashi.dar[r] = 0;
			musashi.dar[8+r] = 0x128;
		}
		for r in rs {
			match r {
				(Register::D0, Bitpattern(bp)) => musashi.dar[0] = bp,
				(Register::D1, Bitpattern(bp)) => musashi.dar[1] = bp,
				(Register::D2, Bitpattern(bp)) => musashi.dar[2] = bp,
				(Register::D3, Bitpattern(bp)) => musashi.dar[3] = bp,
				(Register::D4, Bitpattern(bp)) => musashi.dar[4] = bp,
				(Register::D5, Bitpattern(bp)) => musashi.dar[5] = bp,
				(Register::D6, Bitpattern(bp)) => musashi.dar[6] = bp,
				(Register::D7, Bitpattern(bp)) => musashi.dar[7] = bp,
				// must ensure Addresses are within musashi memory space!
				(Register::A0, Bitpattern(bp)) => musashi.dar[0+8] = bp & mem_mask,
				(Register::A1, Bitpattern(bp)) => musashi.dar[1+8] = bp & mem_mask,
				(Register::A2, Bitpattern(bp)) => musashi.dar[2+8] = bp & mem_mask,
				(Register::A3, Bitpattern(bp)) => musashi.dar[3+8] = bp & mem_mask,
				(Register::A4, Bitpattern(bp)) => musashi.dar[4+8] = bp & mem_mask,
				(Register::A5, Bitpattern(bp)) => musashi.dar[5+8] = bp & mem_mask,
				(Register::A6, Bitpattern(bp)) => musashi.dar[6+8] = bp & mem_mask,
				(Register::A7, Bitpattern(bp)) => musashi.dar[7+8] = bp & STACK_MASK + 8,
				(Register::USP, Bitpattern(bp)) => musashi.inactive_usp = bp & STACK_MASK + 8,
				(Register::SR, Bitpattern(bp)) => musashi.sr_to_flags(bp),
				_ => {
					panic!("No idea how to set {:?}", r.0)
				},
			}
		}

		let mut r68k = musashi.clone(); // so very self-aware!
		let musashi_cycles = reset_and_execute1(&mut musashi);
		let r68k_cycles = r68k.execute1();
		assert_eq!(musashi_cycles, r68k_cycles);
		assert_cores_equal(&musashi, &r68k)
	}

	macro_rules! qc8 {
		($opcode:ident, $fn_name:ident) => (qc!($opcode, MASK_OUT_X_Y, $fn_name, hammer_cores););
		($opcode:ident, $opmask:ident, $fn_name:ident) => (qc!($opcode, $opmask, $fn_name, hammer_cores););
	}
	macro_rules! qc {
		($opcode:ident, $fn_name:ident) => (qc!($opcode, MASK_OUT_X_Y, $fn_name, hammer_cores_even_addresses););
		($opcode:ident, $opmask:ident, $fn_name:ident) => (qc!($opcode, $opmask, $fn_name, hammer_cores_even_addresses););
		($opcode:ident, $opmask:ident, $fn_name:ident, $hammer:ident) => (
		#[test]
		#[ignore]
		#[allow(unused_variables)]
		fn $fn_name() {
			// Musashi isn't thread safe, and the construct with opcode_under_test
			// isn't either. :(
			let mutex = QUICKCHECK_LOCK.lock().unwrap();
			for opcode in opcodes($opmask, $opcode)
			{
				println!("Will hammer {:b}", opcode);
				unsafe {
					// this is because I don't know how to make
					// hammer_cores take the opcode as a parameter and
					// we cannot simply use a closure either; see 
					// https://github.com/BurntSushi/quickcheck/issues/56
					opcode_under_test = opcode;
				}
				QuickCheck::new()
				.gen(StdGen::new(rand::thread_rng(), 256))
				.tests(10)
				.quickcheck($hammer as fn(Vec<(Register, Bitpattern)>) -> bool);
			}
		})
	}

	qc8!(OP_ABCD_8_RR, qc_abcd_rr);
	qc8!(OP_ABCD_8_MM, qc_abcd_mm);

	qc8!(OP_ADD_8_ER_D,  qc_add_8_er_d);
	qc8!(OP_ADD_8_ER_PI, qc_add_8_er_pi);
	qc8!(OP_ADD_8_ER_PD, qc_add_8_er_pd);
	qc8!(OP_ADD_8_ER_AI, qc_add_8_er_ai);
	qc8!(OP_ADD_8_ER_DI, qc_add_8_er_di);
	qc8!(OP_ADD_8_ER_IX, qc_add_8_er_ix);
	qc8!(OP_ADD_8_ER_AW,   MASK_OUT_X, qc_add_8_er_aw);
	qc8!(OP_ADD_8_ER_AL,   MASK_OUT_X, qc_add_8_er_al);
	qc8!(OP_ADD_8_ER_PCDI, MASK_OUT_X, qc_add_8_er_pcdi);
	qc8!(OP_ADD_8_ER_PCIX, MASK_OUT_X, qc_add_8_er_pcix);
	qc8!(OP_ADD_8_ER_IMM,  MASK_OUT_X, qc_add_8_er_imm);

	qc8!(OP_ADD_8_RE_PI, qc_add_8_re_pi);
	qc8!(OP_ADD_8_RE_PD, qc_add_8_re_pd);
	qc8!(OP_ADD_8_RE_AI, qc_add_8_re_ai);
	qc8!(OP_ADD_8_RE_DI, qc_add_8_re_di);
	qc8!(OP_ADD_8_RE_IX, qc_add_8_re_ix);
	qc8!(OP_ADD_8_RE_AW, MASK_OUT_X, qc_add_8_re_aw);
	qc8!(OP_ADD_8_RE_AL, MASK_OUT_X, qc_add_8_re_al);

	qc!(OP_ADD_16_ER_D,  qc_add_16_er_d);
	qc!(OP_ADD_16_ER_A,  qc_add_16_er_a);
	qc!(OP_ADD_16_ER_PI, qc_add_16_er_pi);
	qc!(OP_ADD_16_ER_PD, qc_add_16_er_pd);
	qc!(OP_ADD_16_ER_AI, qc_add_16_er_ai);
	qc!(OP_ADD_16_ER_DI, qc_add_16_er_di);
	qc!(OP_ADD_16_ER_IX, qc_add_16_er_ix);
	qc!(OP_ADD_16_ER_AW,   MASK_OUT_X, qc_add_16_er_aw);
	qc!(OP_ADD_16_ER_AL,   MASK_OUT_X, qc_add_16_er_al);
	qc!(OP_ADD_16_ER_PCDI, MASK_OUT_X, qc_add_16_er_pcdi);
	qc!(OP_ADD_16_ER_PCIX, MASK_OUT_X, qc_add_16_er_pcix);
	qc!(OP_ADD_16_ER_IMM,  MASK_OUT_X, qc_add_16_er_imm);

	qc!(OP_ADD_16_RE_PI, qc_add_16_re_pi);
	qc!(OP_ADD_16_RE_PD, qc_add_16_re_pd);
	qc!(OP_ADD_16_RE_AI, qc_add_16_re_ai);
	qc!(OP_ADD_16_RE_DI, qc_add_16_re_di);
	qc!(OP_ADD_16_RE_IX, qc_add_16_re_ix);
	qc!(OP_ADD_16_RE_AW, MASK_OUT_X, qc_add_16_re_aw);
	qc!(OP_ADD_16_RE_AL, MASK_OUT_X, qc_add_16_re_al);

	qc!(OP_ADD_32_ER_D,  qc_add_32_er_d);
	qc!(OP_ADD_32_ER_A,  qc_add_32_er_a);
	qc!(OP_ADD_32_ER_PI, qc_add_32_er_pi);
	qc!(OP_ADD_32_ER_PD, qc_add_32_er_pd);
	qc!(OP_ADD_32_ER_AI, qc_add_32_er_ai);
	qc!(OP_ADD_32_ER_DI, qc_add_32_er_di);
	qc!(OP_ADD_32_ER_IX, qc_add_32_er_ix);
	qc!(OP_ADD_32_ER_AW,   MASK_OUT_X, qc_add_32_er_aw);
	qc!(OP_ADD_32_ER_AL,   MASK_OUT_X, qc_add_32_er_al);
	qc!(OP_ADD_32_ER_PCDI, MASK_OUT_X, qc_add_32_er_pcdi);
	qc!(OP_ADD_32_ER_PCIX, MASK_OUT_X, qc_add_32_er_pcix);
	qc!(OP_ADD_32_ER_IMM,  MASK_OUT_X, qc_add_32_er_imm);

	qc!(OP_ADD_32_RE_PI, qc_add_32_re_pi);
	qc!(OP_ADD_32_RE_PD, qc_add_32_re_pd);
	qc!(OP_ADD_32_RE_AI, qc_add_32_re_ai);
	qc!(OP_ADD_32_RE_DI, qc_add_32_re_di);
	qc!(OP_ADD_32_RE_IX, qc_add_32_re_ix);
	qc!(OP_ADD_32_RE_AW, MASK_OUT_X, qc_add_32_re_aw);
	qc!(OP_ADD_32_RE_AL, MASK_OUT_X, qc_add_32_re_al);

	qc!(OP_ADDA_16_D,  qc_adda_16_d);
	qc!(OP_ADDA_16_A,  qc_adda_16_a);
	qc!(OP_ADDA_16_PI, qc_adda_16_pi);
	qc!(OP_ADDA_16_PD, qc_adda_16_pd);
	qc!(OP_ADDA_16_AI, qc_adda_16_ai);
	qc!(OP_ADDA_16_DI, qc_adda_16_di);
	qc!(OP_ADDA_16_IX, qc_adda_16_ix);
	qc!(OP_ADDA_16_AW,   MASK_OUT_X, qc_adda_16_aw);
	qc!(OP_ADDA_16_AL,   MASK_OUT_X, qc_adda_16_al);
	qc!(OP_ADDA_16_PCDI, MASK_OUT_X, qc_adda_16_pcdi);
	qc!(OP_ADDA_16_PCIX, MASK_OUT_X, qc_adda_16_pcix);
	qc!(OP_ADDA_16_IMM,  MASK_OUT_X, qc_adda_16_imm);

	qc!(OP_ADDA_32_D,  qc_adda_32_d);
	qc!(OP_ADDA_32_A,  qc_adda_32_a);
	qc!(OP_ADDA_32_PI, qc_adda_32_pi);
	qc!(OP_ADDA_32_PD, qc_adda_32_pd);
	qc!(OP_ADDA_32_AI, qc_adda_32_ai);
	qc!(OP_ADDA_32_DI, qc_adda_32_di);
	qc!(OP_ADDA_32_IX, qc_adda_32_ix);
	qc!(OP_ADDA_32_AW,   MASK_OUT_X, qc_adda_32_aw);
	qc!(OP_ADDA_32_AL,   MASK_OUT_X, qc_adda_32_al);
	qc!(OP_ADDA_32_PCDI, MASK_OUT_X, qc_adda_32_pcdi);
	qc!(OP_ADDA_32_PCIX, MASK_OUT_X, qc_adda_32_pcix);
	qc!(OP_ADDA_32_IMM,  MASK_OUT_X, qc_adda_32_imm);

	qc8!(OP_ADDI_8_D,  MASK_OUT_Y, qc_addi_8_d);
	qc8!(OP_ADDI_8_PI, MASK_OUT_Y, qc_addi_8_pi);
	qc8!(OP_ADDI_8_PD, MASK_OUT_Y, qc_addi_8_pd);
	qc8!(OP_ADDI_8_AI, MASK_OUT_Y, qc_addi_8_ai);
	qc8!(OP_ADDI_8_DI, MASK_OUT_Y, qc_addi_8_di);
	qc8!(OP_ADDI_8_IX, MASK_OUT_Y, qc_addi_8_ix);
	qc8!(OP_ADDI_8_AW, MASK_EXACT, qc_addi_8_aw);
	qc8!(OP_ADDI_8_AL, MASK_EXACT, qc_addi_8_al);

	qc!(OP_ADDI_16_D,  MASK_OUT_Y, qc_addi_16_d);
	qc!(OP_ADDI_16_PI, MASK_OUT_Y, qc_addi_16_pi);
	qc!(OP_ADDI_16_PD, MASK_OUT_Y, qc_addi_16_pd);
	qc!(OP_ADDI_16_AI, MASK_OUT_Y, qc_addi_16_ai);
	qc!(OP_ADDI_16_DI, MASK_OUT_Y, qc_addi_16_di);
	qc!(OP_ADDI_16_IX, MASK_OUT_Y, qc_addi_16_ix);
	qc!(OP_ADDI_16_AW, MASK_EXACT, qc_addi_16_aw);
	qc!(OP_ADDI_16_AL, MASK_EXACT, qc_addi_16_al);

	macro_rules! core_eq {
		($left:ident , $right:ident . $field:ident [ $index:expr ]) => ({
			match (&($left.$field[$index]), &($right.$field[$index])) {
				(left_val, right_val) => {
					if !(*left_val == *right_val) {
						println!("core incoherence: `{}[{}]` differs \
							   ({}: `0x{:x}`, {}: `0x{:x}`)", stringify!($field), $index, stringify!($left), left_val, stringify!($right), right_val);
						return false;
					}
				}
			}
		});
		($left:ident , $right:ident . $field:ident () ?) => ({
			match (&($left.$field()), &($right.$field())) {
				(left_val, right_val) => {
					if !(*left_val == *right_val) {
						println!("core incoherence: `{}()` differs \
							   ({}: `{:?}`, {}: `{:?}`)", stringify!($field), stringify!($left), left_val, stringify!($right), right_val);
						return false;
					}
				}
			}
		});
		($left:ident , $right:ident . $field:ident ()) => ({
			match (&($left.$field()), &($right.$field())) {
				(left_val, right_val) => {
					if !(*left_val == *right_val) {
						println!("core incoherence: `{}()` differs \
							   ({}: `0x{:x}`, {}: `0x{:x}`)", stringify!($field), stringify!($left), left_val, stringify!($right), right_val);
						return false;
					}
				}
			}
		});
		($left:ident , $right:ident . $field:ident) => ({
			match (&($left.$field), &($right.$field)) {
				(left_val, right_val) => {
					if !(*left_val == *right_val) {
						println!("core incoherence: `{}` differs \
							   ({}: `0x{:x}`, {}: `0x{:x}`)", stringify!($field), stringify!($left), left_val, stringify!($right), right_val);
						return false;
					}
				}
			}
		})
	}
	fn assert_cores_equal(musashi: &Core, r68k: &Core) -> bool {
		// check memory accesses match up
		assert_equal(get_ops(), r68k.mem.logger.ops());

		core_eq!(musashi, r68k.pc);
		core_eq!(musashi, r68k.flags() ?);
		core_eq!(musashi, r68k.status_register());
		core_eq!(musashi, r68k.ssp());
		core_eq!(musashi, r68k.usp());
		for i in (0..16).rev() {
			core_eq!(musashi, r68k.dar[i]);
		}
		true
	}

	#[test]
	fn roundtrip_d0() {
		assert_eq!(256, roundtrip_register(Register::D0, 256));
	}

	#[test]
	fn roundtrip_abcd_rr() {
		let pc = 0x40;
		// 0xc101: ABCD		D0, D1
		let mut cpu = Core::new_mem(pc, &[0xc1, 0x01, 0x00, 0x00]);
		cpu.dar[0] = 0x17;
		cpu.dar[1] = 0x27;
		cpu.dar[5] = 0x55555;
		reset_and_execute1(&mut cpu);

		// 17 + 27 is 44
		assert_eq!(0x44, cpu.dar[0]);
		assert_eq!(0x27, cpu.dar[1]);
		assert_eq!(0x55555, cpu.dar[5]);

		let ops = get_ops();
		assert_eq!(1, ops.len());
		assert_eq!(Operation::ReadLong(SUPERVISOR_PROGRAM, pc, 0xc1010000), ops[0]);
	}

	#[test]
	fn compare_abcd_rr() {
		let pc = 0x40;
		// 0xc300: ABCD		D1, D0
		let mut musashi = Core::new_mem(pc, &[0xc3, 0x00]);
		musashi.dar[0] = 0x16;
		musashi.dar[1] = 0x26;

		let mut r68k = musashi.clone(); // so very self-aware!
		reset_and_execute1(&mut musashi);
		r68k.execute1();
		assert_eq!(0x42, r68k.dar[1]);

		assert_cores_equal(&musashi, &r68k);
	}


	#[test]
	#[allow(unused_variables)]
	fn run_abcd_rr_twice() {
		let mutex = MUSASHI_LOCK.lock().unwrap();
		let pc = 0x40;
		// 0xc300: ABCD		D1, D0
		// 0xc302: ABCD		D1, D2
		let mut musashi = Core::new_mem(pc, &[0xc3, 0x00, 0xc3, 0x02]);
		musashi.dar[0] = 0x16;
		musashi.dar[1] = 0x26;
		musashi.dar[2] = 0x31;

		let mut r68k = musashi.clone(); // so very self-aware!

		initialize_musashi(&mut musashi);

		// execute ABCD		D1, D0
		execute1(&mut musashi);
		r68k.execute1();
		assert_eq!(0x42, musashi.dar[1]);
		assert_eq!(0x42, r68k.dar[1]);

		// then execute a second instruction (ABCD D1, D2) on the core
		execute1(&mut musashi);
		r68k.execute1();
		assert_eq!(0x73, musashi.dar[1]);
		assert_eq!(0x73, r68k.dar[1]);

		assert_cores_equal(&musashi, &r68k);
	}

	#[test]
	#[allow(unused_variables)]
	fn compare_address_error_actions() {
		let mutex = MUSASHI_LOCK.lock().unwrap();
		// using an odd absolute address should force an address error
		// opcodes d278,0107 is ADD.W	$0107, D1
		let mut musashi = Core::new_mem(0x40, &[0xd2, 0x78, 0x01, 0x07]);
		let vec3 = 0x200;
		musashi.mem.write_long(SUPERVISOR_PROGRAM, 3*4, vec3);
		musashi.mem.write_long(SUPERVISOR_PROGRAM, vec3, 0xd2780108);
		musashi.dar[15] = 0x100;
		let mut r68k = musashi.clone(); // so very self-aware!
		initialize_musashi(&mut musashi);
		execute1(&mut musashi);
		//execute1(&mut musashi);
		r68k.execute1();
		r68k.execute1();

		assert_cores_equal(&musashi, &r68k);
	}
	#[test]
	#[allow(unused_variables)]
	fn compare_illegal_instruction_actions() {
		let mutex = MUSASHI_LOCK.lock().unwrap();
		// d208 is ADD.B A0,D0, which is illegal
		let mut musashi = Core::new_mem(0x40, &[0xd2, 08]);
		let vec4 = 0x200;
		musashi.mem.write_long(SUPERVISOR_PROGRAM, 4*4, vec4);
		musashi.mem.write_long(SUPERVISOR_PROGRAM, vec4, 0xd2780108);
		musashi.dar[15] = 0x100;
		let mut r68k = musashi.clone(); // so very self-aware!
		initialize_musashi(&mut musashi);
		execute1(&mut musashi);
		//execute1(&mut musashi);
		r68k.execute1();
		//r68k.execute1();

		assert_cores_equal(&musashi, &r68k);
	}
}