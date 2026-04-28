use crate::gekko::condition::BranchControl;
use crate::gekko::instruction::Instruction;
use crate::gekko::lut::*;
use crate::system::{System, SystemId};

#[inline(always)]
pub fn branch<const OP: u32, const SYSTEM: SystemId>(ctx: &mut System<SYSTEM>, instr: Instruction) {
    // Read LR before potentially overwriting LR with CIA+4 (matters for blrl/bctrl)
    let old_lr = ctx.gekko.spr.lr;

    if instr.lk() {
        ctx.gekko.spr.lr = ctx.gekko.cia.wrapping_add(4);
    }

    match OP {
        OP_BX => {
            ctx.gekko.nia = if instr.aa() {
                instr.li() as u32
            } else {
                ctx.gekko.cia.wrapping_add_signed(instr.li())
            }
        }
        OP_BCLRX | OP_BCX | OP_BCCTRX => {
            let ctrl = BranchControl::from_bo(instr.bo());
            tracing::trace!("Branch control: {ctrl:?}");

            if ctrl.should_decrement_ctr() {
                ctx.gekko.spr.ctr = ctx.gekko.spr.ctr.wrapping_sub(1);
            }

            let condition = ctx.gekko.cr.get_bit(instr.bi());
            if !ctrl.should_branch(ctx.gekko.spr.ctr, condition) {
                return;
            }

            match OP {
                OP_BCLRX => ctx.gekko.nia = old_lr,
                OP_BCX => {
                    ctx.gekko.nia = if instr.aa() {
                        instr.bd() as u32
                    } else {
                        ctx.gekko.cia.wrapping_add_signed(instr.bd())
                    }
                }
                OP_BCCTRX => ctx.gekko.nia = ctx.gekko.spr.ctr,
                _ => tracing::error!("missing OP = {OP:#x}"),
            }
        }
        _ => todo!("branch instruction with OP = {OP:#x}"),
    };
}
