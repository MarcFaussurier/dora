use cpu::Reg;
use cpu::arm64::reg::*;
use jit::buffer::Buffer;

pub fn ret() -> u32 {
    cls_uncond_branch_reg(0b0010, 0b11111, 0, REG_LR, 0)
}

pub fn ret_reg(rn: Reg) -> u32 {
    cls_uncond_branch_reg(0b0010, 0b11111, 0, rn, 0)
}

pub fn br(rn: Reg) -> u32 {
    cls_uncond_branch_reg(0b0000, 0b11111, 0, rn, 0)
}

pub fn blr(rn: Reg) -> u32 {
    cls_uncond_branch_reg(0b0001, 0b11111, 0, rn, 0)
}

fn cls_uncond_branch_reg(opc: u32, op2: u32, op3: u32, rn: Reg, op4: u32) -> u32 {
    assert!(fits_u4(opc));
    assert!(fits_u5(op2));
    assert!(fits_u6(op3));
    assert!(rn.is_gpr());
    assert!(fits_u5(op4));

    (0b1101011 as u32) << 25 | opc << 21 | op2 << 16 |
        op3 << 10 | rn.u32() << 5 | op4
}

pub fn nop() -> u32 {
    cls_system(0)
}

fn cls_system(imm: u32) -> u32 {
    assert!(fits_u7(imm));

    0xD503201F | imm << 5
}

pub fn add_imm(sf: u32, rd: Reg, rn: Reg, imm12: u32, shift: u32) -> u32 {
    cls_addsub_imm(sf, 0, 0, shift, imm12, rn, rd)
}

pub fn sub_imm(sf: u32, rd: Reg, rn: Reg, imm12: u32, shift: u32) -> u32 {
    cls_addsub_imm(sf, 1, 0, shift, imm12, rn, rd)
}

fn cls_addsub_imm(sf: u32, op: u32, s: u32, shift: u32, imm12: u32, rn: Reg, rd: Reg) -> u32 {
    assert!(fits_bit(sf));
    assert!(fits_bit(op));
    assert!(fits_bit(s));
    assert!(fits_bit(shift));
    assert!(fits_u12(imm12));
    assert!(rn.is_gpr());
    assert!(rd.is_gpr());

    (0b10001 as u32) << 24 | sf << 31 | op << 30 | s << 29 |
        shift << 22 | imm12 << 10 | rn.u32() << 5 | rd.u32()
}

pub fn add_reg(sf: u32, rd: Reg, rn: Reg, rm: Reg) -> u32 {
    cls_addsub_shreg(sf, 0, 0, Shift::LSL, rm, 0, rn, rd)
}

pub fn sub_reg(sf: u32, rd: Reg, rn: Reg, rm: Reg) -> u32 {
    cls_addsub_shreg(sf, 1, 0, Shift::LSL, rm, 0, rn, rd)
}

pub fn add_shreg(sf: u32, rd: Reg, rn: Reg, rm: Reg, shift: Shift, amount: u32) -> u32 {
    cls_addsub_shreg(sf, 0, 0, shift, rm, amount, rn, rd)
}

pub fn sub_shreg(sf: u32, rd: Reg, rn: Reg, rm: Reg, shift: Shift, amount: u32) -> u32 {
    cls_addsub_shreg(sf, 1, 0, shift, rm, amount, rn, rd)
}

fn cls_addsub_shreg(sf: u32, op: u32, s: u32, shift: Shift, rm: Reg,
                    imm6: u32, rn: Reg, rd: Reg) -> u32 {
    assert!(fits_bit(sf));
    assert!(fits_bit(op));
    assert!(fits_bit(s));
    assert!(!shift.is_ror());
    assert!(rm.is_gpr());
    assert!(rn.is_gpr());
    assert!(rd.is_gpr());
    assert!(fits_u5(imm6));

    0b01011u32 << 24 | sf << 31 | op << 30 | s << 29 |
        shift.u32() << 22 | rm.u32() << 16 | imm6 << 10 | rn.u32() << 5 | rd.u32()
}

pub fn ldrb_ind(rt: Reg, rn: Reg, rm: Reg, extend: LdStExtend, amount: u32) -> u32 {
    cls_ldst_regoffset(0b00, 0, 0b01, rm, extend, amount, rn, rt)
}

pub fn ldrh_ind(rt: Reg, rn: Reg, rm: Reg, extend: LdStExtend, amount: u32) -> u32 {
    cls_ldst_regoffset(0b01, 0, 0b01, rm, extend, amount, rn, rt)
}

pub fn ldrw_ind(rt: Reg, rn: Reg, rm: Reg, extend: LdStExtend, amount: u32) -> u32 {
    cls_ldst_regoffset(0b10, 0, 0b01, rm, extend, amount, rn, rt)
}

pub fn ldrx_ind(rt: Reg, rn: Reg, rm: Reg, extend: LdStExtend, amount: u32) -> u32 {
    cls_ldst_regoffset(0b11, 0, 0b01, rm, extend, amount, rn, rt)
}

pub fn strb_ind(rt: Reg, rn: Reg, rm: Reg, extend: LdStExtend, amount: u32) -> u32 {
    cls_ldst_regoffset(0b00, 0, 0b00, rm, extend, amount, rn, rt)
}

pub fn strh_ind(rt: Reg, rn: Reg, rm: Reg, extend: LdStExtend, amount: u32) -> u32 {
    cls_ldst_regoffset(0b01, 0, 0b00, rm, extend, amount, rn, rt)
}


pub fn strw_ind(rt: Reg, rn: Reg, rm: Reg, extend: LdStExtend, amount: u32) -> u32 {
    cls_ldst_regoffset(0b10, 0, 0b00, rm, extend, amount, rn, rt)
}

pub fn strx_ind(rt: Reg, rn: Reg, rm: Reg, extend: LdStExtend, amount: u32) -> u32 {
    cls_ldst_regoffset(0b11, 0, 0b00, rm, extend, amount, rn, rt)
}

fn cls_ldst_regoffset(size: u32, v: u32, opc: u32, rm: Reg, option: LdStExtend,
                      s: u32, rn: Reg, rt: Reg) -> u32 {
    assert!(fits_u2(size));
    assert!(fits_bit(v));
    assert!(fits_u2(opc));
    assert!(rm.is_gpr());
    assert!(fits_bit(s));
    assert!(rn.is_gpr());
    assert!(rt.is_gpr());

    0b111u32 << 27 | 1u32 << 21 | 0b10u32 << 10 | size << 30 |
        v << 26 | opc << 22 | rm.u32() << 16 | option.u32() << 13 | s << 12 |
        rn.u32() << 5 | rt.u32()
}

pub fn ldrb_imm(rt: Reg, rn: Reg, imm12: u32) -> u32 {
    cls_ldst_regimm(0b00, 0, 0b01, imm12, rn, rt)
}

pub fn ldrh_imm(rt: Reg, rn: Reg, imm12: u32) -> u32 {
    cls_ldst_regimm(0b01, 0, 0b01, imm12, rn, rt)
}

pub fn ldrw_imm(rt: Reg, rn: Reg, imm12: u32) -> u32 {
    cls_ldst_regimm(0b10, 0, 0b01, imm12, rn, rt)
}

pub fn ldrx_imm(rt: Reg, rn: Reg, imm12: u32) -> u32 {
    cls_ldst_regimm(0b11, 0, 0b01, imm12, rn, rt)
}

pub fn strb_imm(rt: Reg, rn: Reg, imm12: u32) -> u32 {
    cls_ldst_regimm(0b00, 0, 0b00, imm12, rn, rt)
}

pub fn strh_imm(rt: Reg, rn: Reg, imm12: u32) -> u32 {
    cls_ldst_regimm(0b01, 0, 0b00, imm12, rn, rt)
}

pub fn strw_imm(rt: Reg, rn: Reg, imm12: u32) -> u32 {
    cls_ldst_regimm(0b10, 0, 0b00, imm12, rn, rt)
}

pub fn strx_imm(rt: Reg, rn: Reg, imm12: u32) -> u32 {
    cls_ldst_regimm(0b11, 0, 0b00, imm12, rn, rt)
}

fn cls_ldst_regimm(size: u32, v: u32, opc: u32, imm12: u32, rn: Reg, rt: Reg) -> u32 {
    assert!(fits_u2(size));
    assert!(fits_bit(v));
    assert!(fits_u2(opc));
    assert!(fits_u12(imm12));
    assert!(rn.is_gpr());
    assert!(rt.is_gpr());

    0b111001u32 << 24 | size << 30 | v << 26 | opc << 22 |
        imm12 << 10 | rn.u32() << 5 | rt.u32()
}

pub fn ldrw_literal(rt: Reg, imm19: i32) -> u32 {
    cls_ld_literal(0b00, 0, imm19, rt)
}

pub fn ldrx_literal(rt: Reg, imm19: i32) -> u32 {
    cls_ld_literal(0b01, 0, imm19, rt)
}

fn cls_ld_literal(opc: u32, v: u32, imm19: i32, rt: Reg) -> u32 {
    assert!(fits_u2(opc));
    assert!(fits_bit(v));
    assert!(fits_i19(imm19));
    assert!(rt.is_gpr());

    let imm = (imm19 as u32) & 0x7FFFF;

    0b011u32 << 27 | opc << 30 | v << 26 | imm << 5 | rt.u32()
}

pub fn and_shreg(sf: u32, rd: Reg, rn: Reg, rm: Reg, shift: Shift, imm6: u32) -> u32 {
    cls_logical_shreg(sf, 0b00, shift, 0, rm, imm6, rn, rd)
}

pub fn bic_shreg(sf: u32, rd: Reg, rn: Reg, rm: Reg, shift: Shift, imm6: u32) -> u32 {
    cls_logical_shreg(sf, 0b00, shift, 1, rm, imm6, rn, rd)
}

pub fn orr_shreg(sf: u32, rd: Reg, rn: Reg, rm: Reg, shift: Shift, imm6: u32) -> u32 {
    cls_logical_shreg(sf, 0b01, shift, 0, rm, imm6, rn, rd)
}

pub fn orn_shreg(sf: u32, rd: Reg, rn: Reg, rm: Reg, shift: Shift, imm6: u32) -> u32 {
    cls_logical_shreg(sf, 0b01, shift, 1, rm, imm6, rn, rd)
}

pub fn eor_shreg(sf: u32, rd: Reg, rn: Reg, rm: Reg, shift: Shift, imm6: u32) -> u32 {
    cls_logical_shreg(sf, 0b10, shift, 0, rm, imm6, rn, rd)
}

pub fn eon_shreg(sf: u32, rd: Reg, rn: Reg, rm: Reg, shift: Shift, imm6: u32) -> u32 {
    cls_logical_shreg(sf, 0b10, shift, 1, rm, imm6, rn, rd)
}

pub fn ands_shreg(sf: u32, rd: Reg, rn: Reg, rm: Reg, shift: Shift, imm6: u32) -> u32 {
    cls_logical_shreg(sf, 0b11, shift, 0, rm, imm6, rn, rd)
}

pub fn bics_shreg(sf: u32, rd: Reg, rn: Reg, rm: Reg, shift: Shift, imm6: u32) -> u32 {
    cls_logical_shreg(sf, 0b11, shift, 1, rm, imm6, rn, rd)
}

fn cls_logical_shreg(sf: u32, opc: u32, shift: Shift, n: u32, rm: Reg,
                     imm6: u32, rn: Reg, rd: Reg) -> u32 {
    assert!(fits_bit(sf));
    assert!(fits_u2(opc));
    assert!(fits_bit(n));
    assert!(rm.is_gpr());
    assert!(fits_u5(imm6));
    assert!(rn.is_gpr());
    assert!(rd.is_gpr());

    0b01010u32 << 24 | sf << 31 | opc << 29 | shift.u32() << 22 |
        n << 21 | rm.u32() << 16 | imm6 << 10 |
        rn.u32() << 5 | rd.u32()
}

pub fn brk(imm16: u32) -> u32 {
    cls_exception(0b001, imm16, 0, 0)
}

fn cls_exception(opc: u32, imm16: u32, op2: u32, ll: u32) -> u32 {
    assert!(fits_u3(opc));
    assert!(fits_u16(imm16));
    assert!(op2 == 0);
    assert!(fits_u2(ll));

    0b11010100u32 << 24 | opc << 21 | imm16 << 5 | op2 << 2 | ll
}

#[derive(Copy, Clone)]
pub enum Extend {
    UXTB, UXTH, LSL, UXTW, UXTX,
    SXTB, SXTH, SXTW, SXTX,
}

impl Extend {
    fn is_ldr(self) -> bool {
        match self {
            Extend::UXTW |
                Extend::LSL |
                Extend::SXTW |
                Extend::SXTX => true,

            _ => false,
        }
    }

    fn u32(self) -> u32 {
        match self {
            Extend::UXTB => 0b000,
            Extend::UXTH => 0b001,
            Extend::LSL  => 0b010,
            Extend::UXTW => 0b010,
            Extend::UXTX => 0b011,
            Extend::SXTB => 0b100,
            Extend::SXTH => 0b101,
            Extend::SXTW => 0b110,
            Extend::SXTX => 0b111,
        }
    }
}

#[derive(Copy, Clone)]
pub enum LdStExtend {
    UXTW, LSL, SXTW, SXTX
}

impl LdStExtend {
    fn u32(self) -> u32 {
        match self {
            LdStExtend::UXTW => 0b010,
            LdStExtend::LSL  => 0b011,
            LdStExtend::SXTW => 0b110,
            LdStExtend::SXTX => 0b111,
        }
    }
}

#[derive(Copy, Clone)]
pub enum Shift {
    LSL, LSR, ASR, ROR
}

impl Shift {
    fn is_ror(self) -> bool {
        match self {
            Shift::ROR => true,
            _ => false,
        }
    }

    fn u32(self) -> u32 {
        match self {
            Shift::LSL => 0,
            Shift::LSR => 1,
            Shift::ASR => 2,
            Shift::ROR => 3,
        }
    }
}

fn fits_bit(imm: u32) -> bool {
    imm < 2
}

fn fits_u2(imm: u32) -> bool {
    imm < 4
}

fn fits_u3(imm: u32) -> bool {
    imm < 8
}

fn fits_u4(imm: u32) -> bool {
    imm < 16
}

fn fits_u5(imm: u32) -> bool {
    imm < 32
}

fn fits_u6(imm: u32) -> bool {
    imm < 64
}

fn fits_u7(imm: u32) -> bool {
    imm < 128
}

fn fits_u12(imm: u32) -> bool {
    imm < 4096
}

fn fits_i12(imm: i32) -> bool {
    -2048 <= imm && imm < 2048
}

fn fits_u16(imm: u32) -> bool {
    imm < 65_536
}

fn fits_i19(imm: i32) -> bool {
    -262_144 <= imm && imm < 262_144
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpu::arm64::reg::*;

    macro_rules! assert_emit {
        (
            $exp:expr;
            $val:expr
        ) => {{
            let exp: u32 = $exp;
            let val: u32 = $val;

            if exp != val {
                panic!("0x{:08X} != 0x{:08X}", exp, val);
            }
        }};
    }

    #[test]
    fn test_fits_bit() {
        assert!(fits_bit(0));
        assert!(fits_bit(1));
        assert!(!fits_bit(2));
    }

    #[test]
    fn test_fits_u4() {
        assert!(fits_u4(0));
        assert!(fits_u4(1));
        assert!(fits_u4(14));
        assert!(fits_u4(15));
        assert!(!fits_u4(16));
        assert!(!fits_u4(17));
    }

    #[test]
    fn test_fits_u5() {
        assert!(fits_u5(0));
        assert!(fits_u5(31));
        assert!(!fits_u5(32));
        assert!(!fits_u5(33));
    }

    #[test]
    fn test_fits_u7() {
        assert!(fits_u7(0));
        assert!(fits_u7(31));
        assert!(fits_u7(126));
        assert!(fits_u7(127));
        assert!(!fits_u7(128));
        assert!(!fits_u7(129));
    }

    #[test]
    fn test_fits_u12() {
        assert!(fits_u12(0));
        assert!(fits_u12(4095));
        assert!(!fits_u12(4096));
        assert!(!fits_u12(4097));
    }

    #[test]
    fn test_fits_i12() {
        assert!(fits_i12(0));
        assert!(fits_i12(-2048));
        assert!(fits_i12(2047));
        assert!(!fits_i12(-2049));
        assert!(!fits_i12(2048));
    }

    #[test]
    fn test_br_blr() {
        assert_emit!(0xd61f0000; br(R0));
        assert_emit!(0xd61f03c0; br(R30));
        assert_emit!(0xd63f0000; blr(R0));
        assert_emit!(0xd63f03c0; blr(R30));
    }

    #[test]
    fn test_nop() {
        assert_emit!(0xd503201f; nop());
    }

    #[test]
    fn test_ret() {
        assert_emit!(0xd65f03c0; ret());
        assert_emit!(0xd65f0000; ret_reg(R0));
        assert_emit!(0xd65f0140; ret_reg(R10));
    }

    #[test]
    fn test_add_imm() {
        assert_emit!(0x11000420; add_imm(0, R0, R1, 1, 0));
        assert_emit!(0x11400c62; add_imm(0, R2, R3, 3, 1));
        assert_emit!(0x91000420; add_imm(1, R0, R1, 1, 0));
        assert_emit!(0x91400c62; add_imm(1, R2, R3, 3, 1));
    }

    #[test]
    fn test_sub_imm() {
        assert_emit!(0x51000420; sub_imm(0, R0, R1, 1, 0));
        assert_emit!(0x51400c62; sub_imm(0, R2, R3, 3, 1));
        assert_emit!(0xd1000420; sub_imm(1, R0, R1, 1, 0));
        assert_emit!(0xd1400c62; sub_imm(1, R2, R3, 3, 1));
    }

    #[test]
    fn test_add_shreg() {
        assert_emit!(0x0b030441; add_shreg(0, R1, R2, R3, Shift::LSL, 1));
        assert_emit!(0x8b0608a4; add_shreg(1, R4, R5, R6, Shift::LSL, 2));
        assert_emit!(0x0b430441; add_shreg(0, R1, R2, R3, Shift::LSR, 1));
        assert_emit!(0x8b8608a4; add_shreg(1, R4, R5, R6, Shift::ASR, 2));
    }

    #[test]
    fn test_sub_shreg() {
        assert_emit!(0x4b030441; sub_shreg(0, R1, R2, R3, Shift::LSL, 1));
        assert_emit!(0xcb0608a4; sub_shreg(1, R4, R5, R6, Shift::LSL, 2));
        assert_emit!(0x4b430441; sub_shreg(0, R1, R2, R3, Shift::LSR, 1));
        assert_emit!(0xcb8608a4; sub_shreg(1, R4, R5, R6, Shift::ASR, 2));
    }

    #[test]
    fn test_add_reg() {
        assert_emit!(0x0b010000; add_reg(0, R0, R0, R1));
        assert_emit!(0x8b010000; add_reg(1, R0, R0, R1));
        assert_emit!(0x0b030041; add_reg(0, R1, R2, R3));
        assert_emit!(0x8b030041; add_reg(1, R1, R2, R3));
    }

    #[test]
    fn test_sub_reg() {
        assert_emit!(0x4b010000; sub_reg(0, R0, R0, R1));
        assert_emit!(0xcb010000; sub_reg(1, R0, R0, R1));
        assert_emit!(0x4b030041; sub_reg(0, R1, R2, R3));
        assert_emit!(0xcb030041; sub_reg(1, R1, R2, R3));
    }

    #[test]
    fn test_ldr_imm() {
        assert_emit!(0x39400420; ldrb_imm(R0, R1, 1));
        assert_emit!(0x39400862; ldrb_imm(R2, R3, 2));

        assert_emit!(0x79400420; ldrh_imm(R0, R1, 1));
        assert_emit!(0x79400862; ldrh_imm(R2, R3, 2));

        assert_emit!(0xb9400420; ldrw_imm(R0, R1, 1));
        assert_emit!(0xb9400862; ldrw_imm(R2, R3, 2));

        assert_emit!(0xf9400420; ldrx_imm(R0, R1, 1));
        assert_emit!(0xf9400862; ldrx_imm(R2, R3, 2));
    }

    #[test]
    fn test_ldr_literal() {
        // forward jump
        assert_emit!(0x18000060; ldrw_literal(R0, 3));
        assert_emit!(0x58000040; ldrx_literal(R0, 2));
        assert_emit!(0x1800007e; ldrw_literal(R30, 3));
        assert_emit!(0x5800005e; ldrx_literal(R30, 2));

        // backward jump
        assert_emit!(0x18ffffe0; ldrw_literal(R0, -1));
        assert_emit!(0x58ffffc0; ldrx_literal(R0, -2));
        assert_emit!(0x18fffffe; ldrw_literal(R30, -1));
        assert_emit!(0x58ffffde; ldrx_literal(R30, -2));
    }

    #[test]
    fn test_ldr_ind() {
        assert_emit!(0x38626820; ldrb_ind(R0, R1, R2, LdStExtend::LSL, 0));
        assert_emit!(0x38656883; ldrb_ind(R3, R4, R5, LdStExtend::LSL, 0));

        assert_emit!(0x78626820; ldrh_ind(R0, R1, R2, LdStExtend::LSL, 0));
        assert_emit!(0x78656883; ldrh_ind(R3, R4, R5, LdStExtend::LSL, 0));

        assert_emit!(0xb8626820; ldrw_ind(R0, R1, R2, LdStExtend::LSL, 0));
        assert_emit!(0xb8657883; ldrw_ind(R3, R4, R5, LdStExtend::LSL, 1));
        assert_emit!(0xb86858e6; ldrw_ind(R6, R7, R8, LdStExtend::UXTW, 1));
        assert_emit!(0xb86bd949; ldrw_ind(R9, R10, R11, LdStExtend::SXTW, 1));

        assert_emit!(0xf8626820; ldrx_ind(R0, R1, R2, LdStExtend::LSL, 0));
        assert_emit!(0xf8657883; ldrx_ind(R3, R4, R5, LdStExtend::LSL, 1));
        assert_emit!(0xf86858e6; ldrx_ind(R6, R7, R8, LdStExtend::UXTW, 1));
        assert_emit!(0xf86bd949; ldrx_ind(R9, R10, R11, LdStExtend::SXTW, 1));
    }

    #[test]
    fn test_str_imm() {
        assert_emit!(0x39000420; strb_imm(R0, R1, 1));
        assert_emit!(0x39000862; strb_imm(R2, R3, 2));

        assert_emit!(0x79000420; strh_imm(R0, R1, 1));
        assert_emit!(0x79000862; strh_imm(R2, R3, 2));

        assert_emit!(0xb9000420; strw_imm(R0, R1, 1));
        assert_emit!(0xb9000862; strw_imm(R2, R3, 2));

        assert_emit!(0xf9000420; strx_imm(R0, R1, 1));
        assert_emit!(0xf9000862; strx_imm(R2, R3, 2));
    }

    #[test]
    fn test_str_ind() {
        assert_emit!(0x38226820; strb_ind(R0, R1, R2, LdStExtend::LSL, 0));
        assert_emit!(0x38256883; strb_ind(R3, R4, R5, LdStExtend::LSL, 0));

        assert_emit!(0x78226820; strh_ind(R0, R1, R2, LdStExtend::LSL, 0));
        assert_emit!(0x78256883; strh_ind(R3, R4, R5, LdStExtend::LSL, 0));

        assert_emit!(0xb8226820; strw_ind(R0, R1, R2, LdStExtend::LSL, 0));
        assert_emit!(0xb8257883; strw_ind(R3, R4, R5, LdStExtend::LSL, 1));
        assert_emit!(0xb82858e6; strw_ind(R6, R7, R8, LdStExtend::UXTW, 1));
        assert_emit!(0xb82bd949; strw_ind(R9, R10, R11, LdStExtend::SXTW, 1));

        assert_emit!(0xf8226820; strx_ind(R0, R1, R2, LdStExtend::LSL, 0));
        assert_emit!(0xf8257883; strx_ind(R3, R4, R5, LdStExtend::LSL, 1));
        assert_emit!(0xf82858e6; strx_ind(R6, R7, R8, LdStExtend::UXTW, 1));
        assert_emit!(0xf82bd949; strx_ind(R9, R10, R11, LdStExtend::SXTW, 1));
    }

    #[test]
    fn test_logical_shreg() {
        assert_emit!(0x0a020420; and_shreg(0, R0, R1, R2, Shift::LSL, 1));
        assert_emit!(0x0a650883; bic_shreg(0, R3, R4, R5, Shift::LSR, 2));
        assert_emit!(0xaa880ce6; orr_shreg(1, R6, R7, R8, Shift::ASR, 3));
        assert_emit!(0xaaeb1149; orn_shreg(1, R9, R10, R11, Shift::ROR, 4));
        assert_emit!(0xca0e15ac; eor_shreg(1, R12, R13, R14, Shift::LSL, 5));
        assert_emit!(0xca711a0f; eon_shreg(1, R15, R16, R17, Shift::LSR, 6));
        assert_emit!(0xea941e72; ands_shreg(1, R18, R19, R20, Shift::ASR, 7));
        assert_emit!(0xeaf726d5; bics_shreg(1, R21, R22, R23, Shift::ROR, 9));
    }

    #[test]
    fn test_brk() {
        assert_emit!(0xd4200000; brk(0));
        assert_emit!(0xd43fffe0; brk(0xFFFF));
    }
}